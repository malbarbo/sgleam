use gleam_core::io::{FileSystemReader, memory::InMemoryFileSystem};
use indoc::formatdoc;
use path_clean::clean;

use std::{fmt::Write as _, path::Path};

use rquickjs::{
    CatchResultExt, CaughtError, Coerced, Context, Ctx, Error, Exception, Function, Module, Object,
    Promise, Result, Runtime, Value,
    context::EvalOptions,
    convert::List,
    function::IntoJsFunc,
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
    qjs::{JS_GetRuntime, JS_SetMaxStackSize},
};

use crate::{
    STACK_SIZE,
    bitmap::load_bitmap,
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::Project,
    host::{check_interrupt, interrupt, now_ms, sleep},
    swriteln,
    text_metrics::{text_height, text_width, text_x_offset, text_y_offset},
};

/// A JavaScript context together with its runtime. A clone shares both,
/// because rquickjs counts the references.
#[derive(Clone)]
pub struct QuickJsEngine {
    context: Context,
}

impl Engine for QuickJsEngine {
    fn new(fs: InMemoryFileSystem) -> std::result::Result<Self, SgleamError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::OnceLock;
            // The handler goes in once for the process, and the result
            // stays, so a second engine hears what the first one heard and
            // never runs quietly with no way to stop it.
            static CTRLC: OnceLock<std::result::Result<(), String>> = OnceLock::new();
            let installed =
                CTRLC.get_or_init(|| ctrlc::set_handler(interrupt).map_err(|err| err.to_string()));
            // What ctrlc says already names its subject, so this hands the
            // message on as it came.
            if let Err(err) = installed {
                return Err(SgleamError::Other(err.clone().into()));
            }
        }

        Ok(QuickJsEngine {
            context: create_context(fs)?,
        })
    }

    fn run_main(
        &self,
        module: &str,
        main: MainFunction,
        show_output: bool,
    ) -> std::result::Result<(), SgleamError> {
        run_main(&self.context, module, main, show_output)
    }

    fn has_var(&self, key: &str) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Object>("repl_vars")
                .and_then(|vars| vars.contains_key(key))
                .unwrap_or(false)
        })
    }

    fn run_tests(&self, modules: &[&str]) -> std::result::Result<(), SgleamError> {
        run_tests(&self.context, modules)
    }

    fn interrupt(&self) {
        interrupt();
    }
}

pub fn create_context(fs: InMemoryFileSystem) -> Result<Context> {
    let runtime = Runtime::new()?;
    runtime.set_interrupt_handler(Some(Box::new(check_interrupt)));
    let context = Context::full(&runtime)?;
    // `Runtime::set_max_stack_size` would not do. Since rquickjs 0.12 it
    // disables the check above 16 MiB, and student recursion needs the whole
    // thread.
    context.with(|ctx| unsafe {
        JS_SetMaxStackSize(
            JS_GetRuntime(ctx.as_raw().as_ptr()),
            (STACK_SIZE - 1024 * 1024) as _,
        );
    });
    runtime.set_loader(FileResolver, ScriptLoader { fs });
    context
        .with(|ctx| {
            seed_bigint_flag(&ctx)?;
            add_console(&ctx)?;
            add_sgleam(&ctx)
        })
        .map(|_| context)
}

fn seed_bigint_flag(ctx: &Ctx) -> Result<()> {
    let flag = gleam_core::javascript::is_bigint_enabled();
    ctx.globals().set("__sgleam_bigint", flag)
}

pub fn run_main(
    context: &Context,
    module: &str,
    main: MainFunction,
    show_output: bool,
) -> std::result::Result<(), SgleamError> {
    let name = main.name();
    let kind = match &main {
        MainFunction::Main => "Main",
        MainFunction::ReplMain { .. } => "ReplMain",
        MainFunction::Smain => "Smain",
        MainFunction::SmainStdin => "SmainStdin",
        MainFunction::SmainStdinLines => "SmainStdinLines",
    };
    // The runtime turns a place in a generated module back into a place in
    // the input, and the lines of each file tell it how. Every file goes in on
    // every run, and not one file as the compiler makes it, so an input that
    // reaches into a module that ran nothing still finds the lines.
    let repl_files = match &main {
        MainFunction::ReplMain { files, .. } => files
            .iter()
            .map(|file| {
                let lines = file
                    .lines
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!(r#"["{}", [{lines}]]"#, file.path)
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    let code = formatdoc! {r#"
        import {{ try_main }} from "./sgleam/sgleam_ffi.mjs";
        import {{ {name} }} from "./{module}.mjs";
        try_main({name}, "{kind}", {show_output}, [{repl_files}]);
        "#
    };
    run_script(context, code)
}

pub fn run_tests(context: &Context, modules: &[&str]) -> std::result::Result<(), SgleamError> {
    let mut src = String::new();
    swriteln!(
        &mut src,
        r#"import {{ run_tests }} from "./sgleam/sgleam_ffi.mjs";"#
    );
    let mut entries = vec![];
    for module in modules {
        let import = module.replace("/", "_");
        swriteln!(&mut src, r#"import * as {import} from "./{module}.mjs";"#);
        // The runtime cannot ask for the file name; the module name is it.
        entries.push(format!(r#"["{module}.gleam", {import}]"#));
    }
    let modules = entries.join(", ");
    swriteln!(
        &mut src,
        "globalThis.tests_passed = run_tests([{modules}]);"
    );
    run_script(context, src)?;
    // The script ran without raising, so it set this.
    let passed = context.with(|ctx| ctx.globals().get("tests_passed").unwrap_or(false));
    if passed {
        Ok(())
    } else {
        Err(SgleamError::TestsFailed)
    }
}

pub fn run_script(context: &Context, source: String) -> std::result::Result<(), SgleamError> {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        // The script imports its neighbors, so it takes the name of one of
        // them, minus the extension. Only a loadable module carries that
        // extension, and without it no import can ever find the script.
        options.filename = Some(Project::out().join("eval_script").into_string());
        // The error rquickjs leaves from an exception only says that there
        // was one, so this catches the exception here. The top level of every
        // module the script imports runs inside this call and never reaches
        // `try_main`, so nothing else is going to say what failed.
        let promise = ctx
            .eval_with_options::<Promise, _>(source, options)
            .catch(&ctx)
            .map_err(script_error)?;
        match promise.finish::<Value>().catch(&ctx) {
            Err(CaughtError::Exception(value)) if is_interrupt(&value) => {
                Err(SgleamError::Interrupted)
            }
            Err(CaughtError::Error(err)) => Err(err.into()),
            Err(_) => Err(SgleamError::UserProgramFailed),
            Ok(_) => Ok(()),
        }
    })
}

/// Says what the error is here and not later. The message of a caught
/// exception comes from the context that caught it, and nothing outside this
/// call can read it.
fn script_error(err: CaughtError<'_>) -> SgleamError {
    match &err {
        CaughtError::Exception(exception) if is_interrupt(exception) => SgleamError::Interrupted,
        // Its `Display` ends in a newline, which the one printing it adds.
        _ => SgleamError::LauncherScript(err.to_string().trim_end().to_string()),
    }
}

/// Returns `true` if the exception is the one QuickJS throws for an
/// interruption, `false` otherwise. That one is its own InternalError, and a
/// panic saying "interrupted" is not.
fn is_interrupt(exception: &Exception) -> bool {
    exception.message() == Some("interrupted".into())
        && exception
            .get("name")
            .is_ok_and(|name: String| name == "InternalError")
}

fn add_console(ctx: &Ctx) -> Result<()> {
    let console = Object::new(ctx.clone())?;
    set_fn(&console, "log", log)?;
    ctx.globals().set("console", console)
}

fn add_sgleam(ctx: &Ctx) -> Result<()> {
    let sgleam = Object::new(ctx.clone())?;
    set_fn(&sgleam, "getline", getline)?;
    set_fn(&sgleam, "print", print_no_newline)?;
    set_fn(&sgleam, "sleep", sleep)?;
    set_fn(&sgleam, "now_ms", now_ms)?;
    // Only the browser draws and reads keys. The library asks whether the
    // property is there, so a native run simply has neither.
    #[cfg(target_arch = "wasm32")]
    set_fn(&sgleam, "draw_svg", crate::host::draw_svg)?;
    #[cfg(target_arch = "wasm32")]
    set_fn(&sgleam, "get_key_event", crate::host::get_key_event)?;
    set_fn(&sgleam, "text_width", text_width)?;
    set_fn(&sgleam, "text_height", text_height)?;
    set_fn(&sgleam, "text_x_offset", text_x_offset)?;
    set_fn(&sgleam, "text_y_offset", text_y_offset)?;
    set_fn(&sgleam, "load_bitmap", |path: String| {
        List(load_bitmap(path))
    })?;
    ctx.globals().set("sgleam", sgleam)
}

/// Sets `name` on `object` to `f`, and names the function `name` as well. A
/// stack trace shows the name of a function, not the name of the property.
fn set_fn<'js, F, P>(object: &Object<'js>, name: &str, f: F) -> Result<()>
where
    F: IntoJsFunc<'js, P> + 'js,
{
    object.set(
        name,
        Function::new(object.ctx().clone(), f)?.with_name(name)?,
    )
}

fn getline() -> Option<String> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    match stdin.read_line(&mut buffer) {
        Ok(0) => None,
        Ok(_) => {
            if buffer.ends_with('\n') {
                buffer.pop();
                if buffer.ends_with('\r') {
                    buffer.pop();
                }
            }
            Some(buffer)
        }
        Err(err) => {
            eprintln!("{err}");
            None
        }
    }
}

fn log(value: Coerced<String>) {
    println!("{}", value.0);
}

fn print_no_newline(s: String) {
    print!("{s}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

#[derive(Debug)]
struct FileResolver;

impl Resolver for FileResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<String> {
        let dir = Path::new(base).parent().ok_or_else(|| {
            Error::new_resolving_message(base, name, format!("no parent for {base}"))
        })?;
        // The generated modules import each other through `..`, and the file
        // system does not resolve that. It looks a path up component by
        // component, exactly as the import writes the path.
        Ok(clean(dir.join(name)).to_string_lossy().into())
    }
}

struct ScriptLoader {
    fs: InMemoryFileSystem,
}

impl Loader for ScriptLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        path: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<Module<'js, Declared>> {
        tracing::debug!("Loading {path}");
        let src = self
            .fs
            .read(path.into())
            .map_err(|err| Error::new_loading_message(path, err.to_string()))?;
        Module::declare(ctx.clone(), path, src)
    }
}

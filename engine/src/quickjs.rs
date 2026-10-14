use gleam_core::io::{FileSystemReader, memory::InMemoryFileSystem};
use indoc::formatdoc;

use std::fmt::Write as _;

use rquickjs::{
    CatchResultExt, CaughtError, Coerced, Context, Ctx, Error, Exception, Function, Module, Object,
    Promise, Result, Runtime, Value,
    context::EvalOptions,
    function::IntoJsFunc,
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
    qjs::{JS_GetRuntime, JS_IsUncatchableError, JS_SetMaxStackSize, JS_SetUncatchableError},
};

use crate::{
    STACK_SIZE,
    engine::{Engine, MainFunction, SGLEAM_FFI},
    error::SgleamError,
    gleam::Project,
    host::{self, check_interrupt, init, now_ms},
    swriteln,
};

/// The engine answers this itself, instead of a number written down beside it.
/// A number kept by hand goes on standing long after the build it named is
/// gone.
pub fn version() -> &'static str {
    // SAFETY: quickjs hands back a pointer to a string constant of its own,
    // which is there before any runtime is and outlives everything reading it.
    let version = unsafe { std::ffi::CStr::from_ptr(rquickjs::qjs::JS_GetVersion()) };
    version.to_str().unwrap_or("unknown")
}

/// A JavaScript context together with its runtime. A clone shares both,
/// because rquickjs counts the references.
#[derive(Clone)]
pub struct QuickJsEngine {
    context: Context,
}

impl Engine for QuickJsEngine {
    fn new(fs: InMemoryFileSystem) -> std::result::Result<Self, SgleamError> {
        init()?;
        Ok(QuickJsEngine {
            context: create_context(fs)?,
        })
    }

    fn run_main(&self, module: &str, main: MainFunction) -> std::result::Result<(), SgleamError> {
        run_main(&self.context, module, main)
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
}

fn create_context(fs: InMemoryFileSystem) -> Result<Context> {
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

fn run_main(
    context: &Context,
    module: &str,
    main: MainFunction,
) -> std::result::Result<(), SgleamError> {
    let name = main.name();
    let kind = main.kind();
    let show_output = main.show_output();
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
        import {{ try_main }} from "{SGLEAM_FFI}";
        import {{ {name} }} from "./{module}.mjs";
        try_main({name}, "{kind}", {show_output}, [{repl_files}]);
        "#
    };
    run_script(context, code)
}

fn run_tests(context: &Context, modules: &[&str]) -> std::result::Result<(), SgleamError> {
    let mut src = String::new();
    swriteln!(&mut src, r#"import {{ run_tests }} from "{SGLEAM_FFI}";"#);
    let mut entries = vec![];
    // Numbered, as a module name holds what a JavaScript name cannot.
    for (n, module) in modules.iter().enumerate() {
        swriteln!(&mut src, r#"import * as m{n} from "./{module}.mjs";"#);
        // The runtime cannot ask for the file name; the module name is it.
        entries.push(format!(r#"["{module}.gleam", m{n}]"#));
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

fn run_script(context: &Context, source: String) -> std::result::Result<(), SgleamError> {
    let result = eval_script(context, source);
    // A world that ended, or that Ctrl-C stopped, leaves its display open.
    crate::host::close_frames();
    result
}

fn eval_script(context: &Context, source: String) -> std::result::Result<(), SgleamError> {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        // The script imports its neighbors, so it takes the name of one of
        // them, minus the extension. Only a loadable module carries that
        // extension, and without it no import can ever find the script.
        options.filename = Some(Project::out_file("eval_script").into_string());
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
/// interruption, `false` otherwise. That one is the only one it marks
/// uncatchable, which is also what carries it out through the `catch` of a
/// test that fails.
fn is_interrupt(exception: &Exception) -> bool {
    // SAFETY: the exception owns the value for the whole of the call.
    unsafe { JS_IsUncatchableError(exception.as_value().as_raw()) }
}

fn add_console(ctx: &Ctx) -> Result<()> {
    let console = Object::new(ctx.clone())?;
    set_fn(&console, "log", log)?;
    ctx.globals().set("console", console)
}

fn add_sgleam(ctx: &Ctx) -> Result<()> {
    let sgleam = Object::new(ctx.clone())?;
    set_fn(&sgleam, "getline", getline)?;
    set_fn(&sgleam, "sleep", sleep)?;
    set_fn(&sgleam, "now_ms", now_ms)?;
    set_fn(&sgleam, "wait_event", wait_event)?;
    set_fn(&sgleam, "open_mworld", open_mworld)?;
    set_fn(&sgleam, "wait_mworld_event", wait_mworld_event)?;
    // Only the browser draws an svg in the repl. The library asks whether the
    // property is there, so a native run simply has none.
    #[cfg(target_arch = "wasm32")]
    set_fn(&sgleam, "draw_svg", crate::host::draw_svg)?;
    #[cfg(not(target_arch = "wasm32"))]
    set_fn(&sgleam, "print_image", crate::scene::print_image)?;
    set_fn(&sgleam, "show_image", crate::scene::show_image)?;
    set_fn(
        &sgleam,
        "show_player_image",
        crate::scene::show_player_image,
    )?;
    set_fn(&sgleam, "close_display", crate::host::close_frames)?;
    set_fn(&sgleam, "text_metrics", crate::scene::text_metrics)?;
    set_fn(&sgleam, "load_image", crate::scene::load_image)?;
    set_fn(&sgleam, "image_to_svg", crate::scene::to_svg)?;
    set_fn(&sgleam, "path_box", crate::scene::path_box)?;
    set_fn(&sgleam, "same_pixels", crate::scene::same_pixels)?;
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

/// `system.sleep`, which stops the program when an interruption arrives. The
/// interrupt handler runs only after many operations, and a loop that mostly
/// sleeps runs few of them.
fn sleep(ctx: Ctx<'_>, ms: u64) -> Result<()> {
    host::sleep(ms);
    stop_on_interrupt(&ctx)
}

/// `system.wait_event`, which stops the program on an interruption, as `sleep`
/// does.
fn wait_event(ctx: Ctx<'_>, ms: u64) -> Result<Vec<String>> {
    let event = host::wait_event(ms);
    stop_on_interrupt(&ctx)?;
    Ok(event)
}

fn open_mworld(
    ctx: Ctx<'_>,
    width: f32,
    height: f32,
    min_players: u32,
    max_players: u32,
) -> Result<Vec<String>> {
    host::open_mworld(width, height, min_players, max_players)
        .map_err(|error| Exception::throw_message(&ctx, &error))
}

fn wait_mworld_event(ctx: Ctx<'_>, ms: u64) -> Result<Vec<String>> {
    let event = host::wait_mworld_event(ms);
    stop_on_interrupt(&ctx)?;
    Ok(event)
}

fn stop_on_interrupt(ctx: &Ctx<'_>) -> Result<()> {
    if !check_interrupt() {
        return Ok(());
    }
    // The handler stops a program with an error that no catch takes, and
    // is_interrupt looks for that mark.
    let error = Exception::from_message(ctx.clone(), "interrupted")?;
    // SAFETY: the error is a live object of this context.
    unsafe { JS_SetUncatchableError(ctx.as_raw().as_ptr(), error.as_value().as_raw()) };
    Err(ctx.throw(error.into_value()))
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
    host::print_line(&value.0);
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
        resolve_path(base, name).ok_or_else(|| {
            Error::new_resolving_message(base, name, format!("no parent for {base}"))
        })
    }
}

/// The path that `name`, imported from the module at `base`, names, or `None`
/// if `base` has no directory. The generated modules import each other through
/// `..`, and the file system does not resolve that. It looks a path up
/// component by component, exactly as the import writes the path. The path
/// separates with `/` on every system, as a stack trace shows it to the user.
fn resolve_path(base: &str, name: &str) -> Option<String> {
    let (dir, _) = base.rsplit_once('/')?;
    // An absolute name stands on its own, as the prelude is imported.
    let dir = if name.starts_with('/') { "" } else { dir };
    let mut parts: Vec<&str> = dir.split('/').collect();
    for part in name.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.len() > 1 {
                    parts.pop();
                }
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
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

#[cfg(test)]
mod tests {
    use super::resolve_path;

    #[test]
    fn an_import_resolves_to_a_path_separated_with_slashes() {
        let resolve = |base, name| resolve_path(base, name).unwrap();
        assert_eq!(resolve("/build/eval_script", "./b.mjs"), "/build/b.mjs");
        assert_eq!(
            resolve("/build/sub/d.mjs", "../gleam/list.mjs"),
            "/build/gleam/list.mjs"
        );
        assert_eq!(
            resolve("/build/b.mjs", "/build/prelude.mjs"),
            "/build/prelude.mjs"
        );
        // A `..` does not climb above the root.
        assert_eq!(resolve("/build/b.mjs", "../../x.mjs"), "/x.mjs");
        assert_eq!(resolve_path("eval_script", "./b.mjs"), None);
    }
}

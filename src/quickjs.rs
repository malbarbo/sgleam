use camino::Utf8Path;
use gleam_core::io::{memory::InMemoryFileSystem, FileSystemReader};
use indoc::formatdoc;

use std::{
    fmt::Write as _,
    io::Write as _,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use rquickjs::{
    context::EvalOptions,
    loader::{Loader, Resolver},
    module::Declared,
    qjs::{JSValue, JS_FreeCString, JS_ToCStringLen},
    Array, CatchResultExt, CaughtError, Context, Ctx, Error, Function, Module, Object, Promise,
    Result, Runtime, Value,
};

use crate::{
    engine::{Engine, MainFunction},
    gleam::Project,
    swriteln, STACK_SIZE,
};

#[derive(Clone)]
pub struct QuickJsEngine {
    context: Context,
    fs: InMemoryFileSystem,
}

impl Engine for QuickJsEngine {
    fn new(fs: InMemoryFileSystem) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        ctrlc::set_handler(interrupt).expect("Add crtlc handlers");

        QuickJsEngine {
            context: create_context(fs.clone(), Project::out().into()).unwrap(),
            fs,
        }
    }

    fn run_main(&self, module: &str, main: MainFunction, show_output: bool) {
        run_main(&self.context, module, main, show_output);
    }

    fn has_var(&self, index: usize) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Array>("repl_vars")
                .map(|a| index < a.len())
                .unwrap_or(false)
        })
    }

    fn run_tests(&self, modules: &[&str]) {
        run_tests(&self.context, modules);
    }

    // FIXME: handle more than one instance?
    fn interrupt(&self) {
        interrupt();
    }
}

impl QuickJsEngine {
    pub fn dump_module(&self, module: &str) {
        let mut path = String::from("/build/");
        path.push_str(module);
        path.push_str(".mjs");
        let content = self.fs.read(Utf8Path::new(&path)).unwrap();
        println!("{path}\n{content}");
    }
}

static STOP: AtomicBool = AtomicBool::new(false);

pub fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn import_check_interrupt() -> bool;
}

#[cfg(target_arch = "wasm32")]
fn check_interrupt() -> bool {
    unsafe { import_check_interrupt() }
}

#[cfg(not(target_arch = "wasm32"))]
fn check_interrupt() -> bool {
    STOP.swap(false, Ordering::Relaxed)
}

pub fn create_context(fs: InMemoryFileSystem, base: PathBuf) -> Result<Context> {
    let runtime = Runtime::new()?;
    runtime.set_max_stack_size(STACK_SIZE - 1024 * 1024);
    runtime.set_interrupt_handler(Some(Box::new(check_interrupt)));
    let context = Context::full(&runtime)?;
    runtime.set_loader(FileResolver { base, first: false }, ScriptLoader { fs });
    context.with(|ctx| add_console(&ctx)).map(|_| context)
}

pub fn run_main(context: &Context, module: &str, main: MainFunction, show_output: bool) {
    let name = main.name();
    let code = formatdoc! {r#"
        import {{ try_main }} from "./sgleam/sgleam_ffi.mjs";
        import {{ {name} }} from "./{module}.mjs";
        try_main({name}, "{main:?}", {show_output});
        "#
    };
    run_script(context, code)
}

pub fn run_tests(context: &Context, modules: &[&str]) {
    let mut src = String::new();
    swriteln!(
        &mut src,
        r#"import {{ run_tests }} from "./sgleam/sgleam_ffi.mjs";"#
    );
    let mut imports = vec![];
    for module in modules {
        let import = module.replace("/", "_");
        swriteln!(&mut src, r#"import * as {import} from "./{module}.mjs";"#);
        imports.push(import);
    }
    let modules = imports.join(", ");
    swriteln!(&mut src, "run_tests([{modules}]);");
    run_script(context, src)
}

pub fn run_script(context: &Context, source: String) {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        match ctx
            .eval_with_options::<Promise, _>(source, options)
            .catch(&ctx)
        {
            Err(err) => js_show_error(err),
            Ok(v) => match v.finish::<Value>().catch(&ctx) {
                Err(CaughtError::Exception(value))
                    if value.message() == Some("interrupted".into()) =>
                {
                    println!("Interrupted.");
                }
                Err(err) => js_show_error(err),

                Ok(_) => (),
            },
        }
    });
}

fn js_show_error(err: CaughtError) {
    // TODO: bump the exception up?
    eprintln!("{}", err);
}

fn add_console(ctx: &Ctx) -> Result<()> {
    let global = ctx.globals();
    let console = Object::new(ctx.clone())?;
    console.set("log", Function::new(ctx.clone(), log)?.with_name("log")?)?;
    console.set(
        "getline",
        Function::new(ctx.clone(), getline)?.with_name("getline")?,
    )?;

    global.set("console", console)?;
    Ok(())
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
            eprintln!("{}", err);
            None
        }
    }
}

fn log(value: Value) {
    // FIXME: remove unsafe use
    // adapted from rquickjs::String::to_string
    pub struct MyValue<'js> {
        pub ctx: Ctx<'js>,
        pub value: JSValue,
    }
    // the ctx and value fields in Value are pub(crate), so we make
    // this transmute to access the fields
    let value: MyValue = unsafe { std::mem::transmute(value) };
    let mut len = std::mem::MaybeUninit::uninit();
    let ptr =
        unsafe { JS_ToCStringLen(value.ctx.as_raw().as_ptr(), len.as_mut_ptr(), value.value) };
    assert!(!ptr.is_null());
    let len = unsafe { len.assume_init() };
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };
    std::io::stdout().write_all(bytes).unwrap();
    println!();
    unsafe { JS_FreeCString(value.ctx.as_raw().as_ptr(), ptr) };
}

#[derive(Debug)]
struct FileResolver {
    pub(crate) base: PathBuf,
    pub(crate) first: bool,
}

impl Resolver for FileResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        let result = if self.first {
            // FIXME: remove this first hack
            self.first = false;
            self.base.join(name)
        } else if base == "eval_script" {
            self.base.join(name.strip_prefix("./").unwrap_or(name))
        } else {
            resolve_path(
                &Path::new(base)
                    .parent()
                    .ok_or_else(|| {
                        Error::new_resolving_message(base, name, format!("no parent for {base}"))
                    })?
                    .join(name),
            )
        };
        Ok(result.to_string_lossy().into())
    }
}

fn resolve_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = components.last() {
                    components.pop();
                }
            }
            Component::CurDir => {}
            _ => {
                components.push(component);
            }
        }
    }

    components.iter().collect()
}

struct ScriptLoader {
    fs: InMemoryFileSystem,
}

impl Loader for ScriptLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, path: &str) -> Result<Module<'js, Declared>> {
        tracing::debug!("Loading {path}");
        let src = self
            .fs
            .read(path.into())
            .map_err(|err| Error::new_loading_message(path, err.to_string()))?;
        Module::declare(ctx.clone(), path, src)
    }
}

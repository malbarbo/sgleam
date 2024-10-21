use camino::Utf8Path;

use gleam_core::io::{memory::InMemoryFileSystem, FileSystemReader};

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use rquickjs::{
    context::EvalOptions,
    loader::{Loader, Resolver},
    module::Declared,
    qjs::{JSValue, JS_FreeCString, JS_ToCStringLen},
    CatchResultExt, CaughtError, Context, Ctx, Function, Module, Object, Promise, Result, Runtime,
    Value,
};

use crate::STACK_SIZE;

pub fn create_js_context(fs: InMemoryFileSystem, base: PathBuf) -> Context {
    let runtime = Runtime::new().unwrap();
    runtime.set_max_stack_size(STACK_SIZE);
    let context = Context::full(&runtime).unwrap();
    runtime.set_loader(FileResolver { base, first: false }, ScriptLoader { fs });
    context.with(|ctx| {
        add_console_log(&ctx);
    });
    context
}

pub fn run_js(context: &Context, source: String) {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        match ctx
            .eval_with_options::<Promise, _>(source, options)
            .catch(&ctx)
        {
            Err(err) => show_js_error(err),
            Ok(v) => {
                if let Err(err) = v.finish::<Value>().catch(&ctx) {
                    show_js_error(err)
                }
            }
        }
    });
}

fn show_js_error(err: CaughtError) {
    eprintln!("{}", err);
    std::process::exit(1);
}

fn add_console_log(ctx: &Ctx) {
    let global = ctx.globals();
    let console = Object::new(ctx.clone()).unwrap();
    console
        .set(
            "log",
            Function::new(ctx.clone(), log)
                .unwrap()
                .with_name("log")
                .unwrap(),
        )
        .unwrap();
    global.set("console", console).unwrap();
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
            let basep = Path::new(base).parent().unwrap();
            if let Some(name) = name.strip_prefix("./") {
                basep.join(name)
            } else if let Some(name) = name.strip_prefix("../") {
                basep.parent().unwrap().join(name)
            } else {
                basep.parent().unwrap().join(name)
            }
        };
        Ok(result.to_str().unwrap().into())
    }
}

struct ScriptLoader {
    fs: InMemoryFileSystem,
}

impl Loader for ScriptLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, path: &str) -> Result<Module<'js, Declared>> {
        // TODO: add tracing
        Module::declare(
            ctx.clone(),
            path,
            self.fs.read(Utf8Path::new(path)).unwrap(),
        )
    }
}

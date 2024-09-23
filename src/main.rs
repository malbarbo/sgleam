use camino::{Utf8Path, Utf8PathBuf};
use gleam_core::{
    build::{
        Mode, NullTelemetry, PackageCompiler, StaleTracker, Target, TargetCodegenConfiguration,
    },
    config::PackageConfig,
    io::{memory::InMemoryFileSystem, CommandExecutor, FileSystemReader, FileSystemWriter},
    javascript::PRELUDE,
    uid::UniqueIdGenerator,
    warning::WarningEmitter,
    Error,
};
use rquickjs::{
    context::EvalOptions, loader::{Loader, Resolver}, qjs::{JSValue, JS_FreeCString, JS_ToCStringLen}, Context, Ctx, Function, Module, Object, Promise, Runtime, Value
};
use sgleam::{stderr_buffer_writer, ConsoleWarningEmitter};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::exit,
    sync::Arc,
};
use tar::Archive;

const GLEAM_STDLIB: &[u8] = include_bytes!("../gleam-stdlib.tar");

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Wrong number of arguments.");
        exit(1);
    }

    let input = Utf8Path::new(&args[1]);
    if !input.is_file() {
        eprintln!("{input}: does not exist or is not a file.");
        exit(1);
    }

    if input.extension() != Some("gleam") {
        eprintln!("{input}: is not a gleam file.");
        exit(1);
    }

    let mut fs = InMemoryFileSystem::new();

    match build(&mut fs, input) {
        Err(err) => {
            let buffer_writer = stderr_buffer_writer();
            let mut buffer = buffer_writer.buffer();
            err.pretty(&mut buffer);
            buffer_writer
                .print(&buffer)
                .expect("Writing warning to stderr");
        }
        Ok(file) => run(fs.clone(), &file),
    }
}

fn run(fs: InMemoryFileSystem, path: &Utf8Path) {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let resolver = FileResolver {
        base: path.as_std_path().parent().unwrap().to_path_buf(),
        num: 0,
    };
    rt.set_loader(resolver, ScriptLoader { fs: fs.clone() });
    ctx.with(|ctx| {
        add_console_log(&ctx);
        let mut options = EvalOptions::default();
        options.global = false;
        match ctx.eval_with_options::<Promise, _>(fs.read(path).unwrap(), options) {
            Err(rquickjs::Error::Exception) => {
                eprintln!("{}", ctx.catch().as_exception().unwrap());
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("{:?}", e);
                std::process::exit(1);
            }
            Ok(v) => {
                v.finish::<Value>().unwrap();
            }
        }
    });
}

fn build(project: &mut InMemoryFileSystem, input: &Utf8Path) -> Result<Utf8PathBuf, Error> {
    let root = Utf8Path::new("/");
    let src = root.join("src");
    extract_tar(project, Archive::new(GLEAM_STDLIB), &src)?;
    copy_file(
        project,
        input,
        &src.join(input.file_name().expect("A file name")),
    );
    compile(project, &root, input.file_stem().expect("A file steam"))
}

fn extract_tar<T: FileSystemWriter>(
    fs: &mut T,
    mut arch: Archive<&[u8]>,
    to: &Utf8Path,
) -> Result<(), Error> {
    let mut buf = vec![];
    for entry in arch.entries().map_err(to_error_stdio)? {
        let mut entry = entry.map_err(to_error_stdio)?;
        let is_dir = entry.header().entry_type().is_dir();
        let entry_path = entry.path().map_err(to_error_stdio)?.into_owned();
        let entry_path = Utf8PathBuf::from_path_buf(entry_path).map_err(to_error_nonutf8_path)?;
        let path = to.join(entry_path);
        if is_dir {
            fs.mkdir(&path)?;
        } else {
            buf.clear();
            entry.read_to_end(&mut buf).map_err(to_error_stdio)?;
            fs.write_bytes(&path, &buf)?;
        }
    }
    Ok(())
}

fn copy_file<T: FileSystemWriter>(fs: &mut T, from: &Utf8Path, to: &Utf8Path) {
    fs.write_bytes(to, &std::fs::read(from).unwrap()).unwrap()
}

fn compile<IO>(project: &mut IO, root: &Utf8Path, name: &str) -> Result<Utf8PathBuf, Error>
where
    IO: FileSystemReader + FileSystemWriter + CommandExecutor + Clone,
{
    let config = PackageConfig {
        target: Target::JavaScript,
        ..Default::default()
    };

    let out = root.join("build");
    let prelude = out.join("prelude.mjs");
    let main = out.join("main.mjs");
    let main_content = &format!("import {{ main }} from \"./{name}.mjs\";\nmain();");

    let target = TargetCodegenConfiguration::JavaScript {
        emit_typescript_definitions: false,
        prelude_location: prelude.clone(),
    };

    let mut compiler = PackageCompiler::new(
        &config,
        Mode::Dev,
        &root,
        &out,
        &out,
        &target,
        UniqueIdGenerator::new(),
        project.clone(),
    );
    compiler.write_metadata = false;

    compiler
        .compile(
            &WarningEmitter::new(Arc::new(ConsoleWarningEmitter)),
            &mut im::HashMap::new(),
            &mut im::HashMap::new(),
            &mut StaleTracker::default(),
            &mut HashSet::new(),
            &NullTelemetry,
        )
        .into_result()
        .and_then(|_| project.write(&prelude, PRELUDE))
        .and_then(|_| project.write(&main, main_content))
        .map(|_| main)
}

fn to_error_stdio(err: std::io::Error) -> Error {
    Error::StandardIo {
        action: gleam_core::error::StandardIoAction::Read,
        err: Some(err.kind()),
    }
}

fn to_error_nonutf8_path(path: PathBuf) -> Error {
    Error::NonUtf8Path { path }
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
    base: PathBuf,
    num: u32,
}

impl Resolver for FileResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> rquickjs::Result<String> {
        let result = if self.num == 0 {
            self.num += 1;
            self.base.join(name)
        } else if base == "eval_script" {
            Path::new("/build/").join(name)
        } else {
            let basep = Path::new(base).parent().unwrap();
            if name.starts_with("./") {
                basep.join(&name[2..])
            } else if name.starts_with("../") {
                basep.parent().unwrap().join(&name[3..])
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
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        path: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        Module::declare(ctx.clone(), path, self.fs.read(Utf8Path::new(path)).unwrap())
    }
}

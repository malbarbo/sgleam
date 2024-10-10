use camino::{Utf8Path, Utf8PathBuf};
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::{
    build::{
        Mode, NullTelemetry, PackageCompiler, StaleTracker, Target, TargetCodegenConfiguration,
    },
    config::PackageConfig,
    io::{memory::InMemoryFileSystem, FileSystemReader, FileSystemWriter},
    javascript::PRELUDE,
    uid::UniqueIdGenerator,
    warning::WarningEmitter,
    Error,
};
use rquickjs::{
    context::EvalOptions,
    loader::{Loader, Resolver},
    qjs::{JSValue, JS_FreeCString, JS_ToCStringLen},
    Context, Ctx, Function, Module, Object, Promise, Runtime, Value,
};
use sgleam::{repl::ReplReader, stderr_buffer_writer, ConsoleWarningEmitter};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
    time::SystemTime,
};
use tar::Archive;

/// The student version of gleam.
#[derive(Parser)]
#[command(
    about,
    styles = Styles::styled()
        .header(styling::AnsiColor::Yellow.on_default())
        .usage(styling::AnsiColor::Yellow.on_default())
        .literal(styling::AnsiColor::Green.on_default())
)]
struct Cli {
    /// Go to iterative mode.
    #[arg(short, group = "cmd")]
    interative: bool,
    /// Run tests.
    #[arg(short, group = "cmd")]
    test: bool,
    /// Format source code.
    #[arg(short, group = "cmd")]
    format: bool,
    /// Print version.
    #[arg(short, long)]
    version: bool,
    /// The program file.
    // TODO: allow multiple files
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return;
    }

    let path = if let Some(path) = cli.path {
        path
    } else {
        repl(&mut Project::new(), None);
        return;
    };

    let input = Utf8Path::from_path(&path).unwrap();
    if !input.is_file() {
        eprintln!("{input}: does not exist or is not a file.");
        exit(1);
    }

    if input.extension() != Some("gleam") {
        eprintln!("{input}: is not a gleam file.");
        exit(1);
    }

    if cli.format {
        if let Err(err) = sgleam::format::run(false, false, vec![input.as_str().into()]) {
            show_gleam_error(err);
            exit(1)
        }
        return;
    }

    let mut project = Project::new();

    match build(&mut project, input, cli.test) {
        Err(err) => show_gleam_error(err),
        Ok(_) => {
            if cli.interative {
                repl(&mut project, Some(input.file_stem().unwrap()));
            } else {
                let source = project.fs.read(&Project::main()).unwrap();
                run_js(&create_js_context(project.fs.clone()), source)
            }
        }
    }
}

struct Project {
    fs: InMemoryFileSystem,
}

impl Project {
    fn new() -> Project {
        let mut project = Project {
            fs: InMemoryFileSystem::new(),
        };
        extract_tar(
            &mut project.fs,
            Archive::new(sgleam::GLEAM_STDLIB),
            Project::source(),
        )
        .expect("Extracting gleam-stdlib.tar");
        project.write_source("sgleam.gleam", sgleam::SGLEAM_GLEAM);
        project.write_source("sgleam_ffi.mjs", sgleam::SGLEAM_FFI_MJS);
        project.write_out("prelude.mjs", PRELUDE);
        project
    }

    fn root() -> &'static Utf8Path {
        "/".into()
    }

    fn source() -> &'static Utf8Path {
        "/src".into()
    }

    fn out() -> &'static Utf8Path {
        "/build".into()
    }

    fn main() -> &'static Utf8Path {
        "/build/main.mjs".into()
    }

    fn prelude() -> &'static Utf8Path {
        "/build/prelude.mjs".into()
    }

    fn write_source(&mut self, name: &str, content: &str) {
        let path = Project::source().join(name);
        self.fs
            .write(&path, content)
            .expect(&format!("Write {path}"));
        self.fs
            .try_set_modification_time(&path, SystemTime::now())
            .expect(&format!("Set modification time {path}"))
    }

    fn write_out(&mut self, name: &str, content: &str) {
        let msg = format!("Writing {name}");
        self.fs
            .write(&Project::out().join(name), content)
            .expect(&msg);
    }
}

fn show_gleam_error(err: Error) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    err.pretty(&mut buffer);
    buffer_writer
        .print(&buffer)
        .expect("Writing warning to stderr");
}

fn repl(project: &mut Project, user_module: Option<&str>) {
    let editor = ReplReader::new().unwrap();
    let context = create_js_context(project.fs.clone());
    for (n, code) in editor.filter(|s| !s.is_empty()).enumerate() {
        let file = format!("repl{n}.gleam");
        write_repl_source(project, &file, &code, user_module);
        match compile(project, &format!("repl{n}"), true, false) {
            Err(err) => show_gleam_error(err),
            Ok(_) => {
                let source = project.fs.read(Project::main()).unwrap();
                run_js(&context, source);
            }
        }
        project
            .fs
            .delete_file(&Project::source().join(file))
            .unwrap();
    }
}

fn write_repl_source(project: &mut Project, file: &str, code: &str, user_module: Option<&str>) {
    let user_module = if let Some(module) = user_module {
        format!("import {module}")
    } else {
        "".into()
    };
    project.write_source(
        file,
        &format!(
            "
{user_module}
import gleam/bit_array
import gleam/bool
import gleam/bytes_builder
import gleam/dict
import gleam/dynamic
import gleam/float
import gleam/function
import gleam/int
import gleam/io
import gleam/iterator
import gleam/list
import gleam/option
import gleam/order
import gleam/pair
import gleam/queue
import gleam/regex
import gleam/result
import gleam/set
import gleam/string
import gleam/string_builder
import gleam/uri
pub fn main() {{
    io.debug({{
{code}
    }})
}}
"
        ),
    );
}

fn create_js_context(fs: InMemoryFileSystem) -> Context {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let resolver = FileResolver {
        base: Project::out().as_std_path().to_path_buf(),
        first: false,
    };
    runtime.set_loader(resolver, ScriptLoader { fs: fs.clone() });
    context.with(|ctx| {
        add_console_log(&ctx);
    });
    context
}

fn run_js(context: &Context, source: String) {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        match ctx.eval_with_options::<Promise, _>(source, options) {
            Err(err) => show_js_error(&ctx, err),
            Ok(v) => {
                if let Err(err) = v.finish::<Value>() {
                    show_js_error(&ctx, err)
                }
            }
        }
    });
}

fn show_js_error(ctx: &Ctx, err: rquickjs::Error) {
    if let rquickjs::Error::Exception = err {
        eprintln!("{:?}", ctx.catch().as_exception());
    } else {
        eprintln!("{}", err);
    }
    std::process::exit(1);
}

fn build(project: &mut Project, input: &Utf8Path, test: bool) -> Result<(), Error> {
    copy_file(
        &mut project.fs,
        input,
        &Project::source().join(input.file_name().expect("A file name")),
    );
    compile(
        project,
        input.file_stem().expect("A file steam"),
        false,
        test,
    )
}

fn extract_tar(
    fs: &mut InMemoryFileSystem,
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

// FIXME: split compilation from generating main.mjs
fn compile(project: &mut Project, name: &str, repl: bool, test: bool) -> Result<(), Error> {
    // TODO: simplify?
    let main_content = if !test {
        &format!(
            "
        import {{ try_main }} from \"./sgleam_ffi.mjs\";
        import {{ main }} from \"./{name}.mjs\";
        try_main(main);
        "
        )
    } else {
        &format!(
            "
        import {{ run_tests }} from \"./sgleam_ffi.mjs\";
        import * as {name} from \"./{name}.mjs\";
        run_tests({name});
        "
        )
    };
    project.write_out("main.mjs", main_content);

    let config = PackageConfig {
        target: Target::JavaScript,
        ..Default::default()
    };

    let target = TargetCodegenConfiguration::JavaScript {
        emit_typescript_definitions: false,
        prelude_location: Project::prelude().into(),
    };

    let mut compiler = PackageCompiler::new(
        &config,
        Mode::Dev,
        Project::root(),
        Project::out(),
        Project::out(),
        &target,
        UniqueIdGenerator::new(),
        project.fs.clone(),
    );

    compiler.write_metadata = true;

    compiler
        .compile(
            &WarningEmitter::new(Rc::new(ConsoleWarningEmitter::with_repl(repl))),
            &mut im::HashMap::new(),
            &mut im::HashMap::new(),
            &mut StaleTracker::default(),
            &mut HashSet::new(),
            &NullTelemetry,
        )
        .into_result()
        .map(|_| ())
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
    first: bool,
}

impl Resolver for FileResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> rquickjs::Result<String> {
        let result = if self.first {
            // FIXME: remove this first hack
            self.first = false;
            self.base.join(name)
        } else if base == "eval_script" {
            Project::out()
                .as_std_path()
                .join(name.strip_prefix("./").unwrap_or(name))
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
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        path: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        // TODO: add tracing
        Module::declare(
            ctx.clone(),
            path,
            self.fs.read(Utf8Path::new(path)).unwrap(),
        )
    }
}

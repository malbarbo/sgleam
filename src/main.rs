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
use rustyline::{error::ReadlineError, DefaultEditor};
use sgleam::{stderr_buffer_writer, ConsoleWarningEmitter};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
};
use tar::Archive;

const GLEAM_STDLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gleam-stdlib.tar"));
const SGLEAM_GLEAM: &str = include_str!("../sgleam.gleam");
const SGLEAM_FFI_MJS: &str = include_str!("../sgleam_ffi.mjs");
const SGLEAM_VERSION: &str = env!("CARGO_PKG_VERSION");
const GLEAM_VERSION: &str = gleam_core::version::COMPILER_VERSION;
const GLEAM_STDLIB_VERSION: &str = "0.40.0";

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
    #[arg(short, group="cmd")]
    interative: bool,
    /// Run tests.
    #[arg(short, group="cmd")]
    test: bool,
    /// Format source code.
    #[arg(short, group="cmd")]
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
        println!("{}", version());
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
                run_js(project.fs)
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
            Archive::new(GLEAM_STDLIB),
            Project::source(),
        )
        .expect("Extracting gleam-stdlib.tar");
        project.write_source("sgleam.gleam", SGLEAM_GLEAM);
        project.write_source("sgleam_ffi.mjs", SGLEAM_FFI_MJS);
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
        let msg = format!("Writing {name}");
        self.fs
            .write(&Project::source().join(name), content)
            .expect(&msg);
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

fn version() -> String {
    format!(
        "sgleam {SGLEAM_VERSION} (using gleam {GLEAM_VERSION} and stdlib {GLEAM_STDLIB_VERSION})"
    )
}

fn repl(project: &mut Project, user_module: Option<&str>) {
    let history = dirs::home_dir().map(|p| p.join(".sgleam_history"));

    let mut rl = DefaultEditor::new().unwrap();
    if let Some(history) = &history {
        let _ = rl.load_history(history);
    }

    println!("Welcome to {}.", version());
    println!("Type \"quit\" to exit.");
    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(input) if input.trim() == "quit" => {
                return;
            }
            Ok(input) => {
                let _ = rl.add_history_entry(&input);
                run_gleam_str(project, &input, user_module);
            }
            Err(ReadlineError::Interrupted) => {}
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    if let Some(history) = &history {
        let _ = rl.save_history(history);
    }
}

fn run_gleam_str(project: &mut Project, code: &str, user_module: Option<&str>) {
    let user_module = if let Some(module) = user_module {
        format!("import {module}")
    } else {
        "".into()
    };
    project.write_source(
        "repl.gleam",
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

    match compile(project, "repl", true, false) {
        Err(err) => show_gleam_error(err),
        Ok(_) => run_js(project.fs.clone()),
    }
}

fn run_js(fs: InMemoryFileSystem) {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let resolver = FileResolver {
        base: Project::out().as_std_path().to_path_buf(),
        first: false,
    };
    rt.set_loader(resolver, ScriptLoader { fs: fs.clone() });
    ctx.with(|ctx| {
        add_console_log(&ctx);
        let mut options = EvalOptions::default();
        options.global = false;
        match ctx.eval_with_options::<Promise, _>(fs.read(Project::main()).unwrap(), options) {
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

    compiler.write_metadata = false;

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
            Project::out().as_std_path().join(name)
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
        Module::declare(
            ctx.clone(),
            path,
            self.fs.read(Utf8Path::new(path)).unwrap(),
        )
    }
}

use camino::{Utf8Path, Utf8PathBuf};
use gleam_core::{
    build::{
        Mode, NullTelemetry, PackageCompiler, StaleTracker, Target, TargetCodegenConfiguration,
        Telemetry,
    },
    config::PackageConfig,
    io::{memory::InMemoryFileSystem, FileSystemWriter},
    javascript::PRELUDE,
    uid::UniqueIdGenerator,
    warning::{WarningEmitter, WarningEmitterIO},
    Error, Warning,
};
use std::{
    collections::HashSet,
    io::{IsTerminal, Read, Write},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant, SystemTime},
};
use tar::Archive;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub struct Project {
    pub fs: InMemoryFileSystem,
}

impl Project {
    pub fn new() -> Project {
        let mut project = Project {
            fs: InMemoryFileSystem::new(),
        };
        extract_tar(
            &mut project.fs,
            Archive::new(crate::GLEAM_STDLIB),
            Project::source(),
        )
        .expect("Extracting gleam-stdlib.tar");
        project.write_source("sgleam/check.gleam", crate::SGLEAM_CHECK);
        project.write_source("sgleam_ffi.mjs", crate::SGLEAM_FFI_MJS);
        project.write_out("prelude.mjs", PRELUDE);
        project
    }

    pub fn root() -> &'static Utf8Path {
        "/".into()
    }

    pub fn source() -> &'static Utf8Path {
        "/src".into()
    }

    pub fn out() -> &'static Utf8Path {
        "/build".into()
    }

    pub fn main() -> &'static Utf8Path {
        "/build/main.mjs".into()
    }

    pub fn prelude() -> &'static Utf8Path {
        "/build/prelude.mjs".into()
    }

    pub fn write_source(&mut self, name: &str, content: &str) {
        let path = Project::source().join(name);
        self.fs
            .write(&path, content)
            .expect(&format!("Write {path}"));
        self.fs
            .try_set_modification_time(&path, SystemTime::now())
            .expect(&format!("Set modification time {path}"))
    }

    pub fn write_out(&mut self, name: &str, content: &str) {
        let msg = format!("Writing {name}");
        self.fs
            .write(&Project::out().join(name), content)
            .expect(&msg);
    }
}

pub fn show_gleam_error(err: Error) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    err.pretty(&mut buffer);
    buffer_writer
        .print(&buffer)
        .expect("Writing warning to stderr");
}

pub fn build(project: &mut Project, input: &Utf8Path, test: bool) -> Result<(), Error> {
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

// FIXME: split compilation from generating main.mjs
pub fn compile(project: &mut Project, name: &str, repl: bool, test: bool) -> Result<(), Error> {
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

fn to_error_stdio(err: std::io::Error) -> Error {
    Error::StandardIo {
        action: gleam_core::error::StandardIoAction::Read,
        err: Some(err.kind()),
    }
}

fn to_error_nonutf8_path(path: PathBuf) -> Error {
    Error::NonUtf8Path { path }
}

// The remaining of this file is copied from gleam project

#[derive(Debug, Default, Clone)]
pub struct Reporter;

impl Reporter {
    pub fn new() -> Self {
        Self
    }
}

impl Telemetry for Reporter {
    fn compiled_package(&self, duration: Duration) {
        print_compiled(duration);
    }

    fn compiling_package(&self, name: &str) {
        print_compiling(name);
    }

    fn checked_package(&self, duration: Duration) {
        print_checked(duration);
    }

    fn checking_package(&self, name: &str) {
        print_checking(name);
    }

    fn downloading_package(&self, name: &str) {
        print_downloading(name)
    }

    fn packages_downloaded(&self, start: Instant, count: usize) {
        print_packages_downloaded(start, count)
    }

    fn resolving_package_versions(&self) {
        print_resolving_versions()
    }

    fn running(&self, name: &str) {
        print_running(name);
    }

    fn waiting_for_build_directory_lock(&self) {
        print_waiting_for_build_directory_lock()
    }
}

pub fn print_published(duration: Duration) {
    print_colourful_prefix("Published", &format!("in {}", seconds(duration)))
}

pub fn print_retired(package: &str, version: &str) {
    print_colourful_prefix("Retired", &format!("{package} {version}"))
}

pub fn print_unretired(package: &str, version: &str) {
    print_colourful_prefix("Unretired", &format!("{package} {version}"))
}

pub fn print_publishing_documentation() {
    print_colourful_prefix("Publishing", "documentation");
}

fn print_downloading(text: &str) {
    print_colourful_prefix("Downloading", text)
}

fn print_waiting_for_build_directory_lock() {
    print_colourful_prefix("Waiting", "for build directory lock")
}

fn print_resolving_versions() {
    print_colourful_prefix("Resolving", "versions")
}

fn print_compiling(text: &str) {
    print_colourful_prefix("Compiling", text)
}

pub(crate) fn print_checking(text: &str) {
    print_colourful_prefix("Checking", text)
}

pub(crate) fn print_compiled(duration: Duration) {
    print_colourful_prefix("Compiled", &format!("in {}", seconds(duration)))
}

pub(crate) fn print_checked(duration: Duration) {
    print_colourful_prefix("Checked", &format!("in {}", seconds(duration)))
}

pub(crate) fn print_running(text: &str) {
    print_colourful_prefix("Running", text)
}

fn print_packages_downloaded(start: Instant, count: usize) {
    let elapsed = seconds(start.elapsed());
    let msg = match count {
        1 => format!("1 package in {elapsed}"),
        _ => format!("{count} packages in {elapsed}"),
    };
    print_colourful_prefix("Downloaded", &msg)
}

pub fn seconds(duration: Duration) -> String {
    format!("{:.2}s", duration.as_millis() as f32 / 1000.)
}

pub fn print_colourful_prefix(prefix: &str, text: &str) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    buffer
        .set_color(
            ColorSpec::new()
                .set_intense(true)
                .set_fg(Some(Color::Magenta)),
        )
        .expect("print_colourful_prefix");
    write!(buffer, "{prefix: >11}").expect("print_colourful_prefix");
    buffer
        .set_color(&ColorSpec::new())
        .expect("print_colourful_prefix");
    writeln!(buffer, " {text}").expect("print_colourful_prefix");
    buffer_writer
        .print(&buffer)
        .expect("print_colourful_prefix");
}

pub fn stderr_buffer_writer() -> BufferWriter {
    // Don't add color codes to the output if standard error isn't connected to a terminal
    BufferWriter::stderr(color_choice())
}

fn colour_forced() -> bool {
    if let Ok(force) = std::env::var("FORCE_COLOR") {
        !force.is_empty()
    } else {
        false
    }
}

fn color_choice() -> ColorChoice {
    if colour_forced() {
        ColorChoice::Always
    } else if std::io::stderr().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConsoleWarningEmitter {
    repl: bool,
}

impl ConsoleWarningEmitter {
    pub fn with_repl(repl: bool) -> ConsoleWarningEmitter {
        ConsoleWarningEmitter { repl }
    }
}

impl WarningEmitterIO for ConsoleWarningEmitter {
    fn emit_warning(&self, warning: Warning) {
        if self.repl {
            if let Warning::Type {
                warning:
                    gleam_core::type_::Warning::UnusedImportedValue { .. }
                    | gleam_core::type_::Warning::UnusedImportedModule { .. }
                    | gleam_core::type_::Warning::UnusedImportedModuleAlias { .. },
                ..
            } = warning
            {
                return;
            }
        }
        let buffer_writer = stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        warning.pretty(&mut buffer);
        buffer_writer
            .print(&buffer)
            .expect("Writing warning to stderr");
    }
}

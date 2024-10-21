use camino::{Utf8Path, Utf8PathBuf};
use gleam_core::{
    build::{
        Mode, Module, NullTelemetry, PackageCompiler, StaleTracker, Target,
        TargetCodegenConfiguration, Telemetry,
    },
    config::PackageConfig,
    error::{FileIoAction, FileKind},
    io::{memory::InMemoryFileSystem, FileSystemWriter},
    javascript::PRELUDE,
    type_::ModuleFunction,
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

impl Default for Project {
    fn default() -> Project {
        let mut project = Project {
            fs: InMemoryFileSystem::new(),
        };
        extract_tar(
            &mut project.fs,
            Archive::new(crate::GLEAM_STDLIB),
            Project::source(),
        )
        .expect("Extract gleam-stdlib.tar");
        project.write_source("sgleam/check.gleam", crate::SGLEAM_CHECK);
        project.write_source("sgleam_ffi.mjs", crate::SGLEAM_FFI_MJS);
        project.write_out("prelude.mjs", PRELUDE);
        project
    }
}

impl Project {
    pub fn root() -> &'static Utf8Path {
        "/".into()
    }

    pub fn source() -> &'static Utf8Path {
        "/src".into()
    }

    pub fn out() -> &'static Utf8Path {
        "/build".into()
    }

    pub fn prelude() -> &'static Utf8Path {
        "/build/prelude.mjs".into()
    }

    pub fn write_source(&mut self, name: &str, content: &str) {
        let path = Project::source().join(name);
        self.fs
            .write(&path, content)
            .expect("Write a file in memory");
        self.fs
            .try_set_modification_time(&path, SystemTime::now())
            .expect("Set modification time of a file in memory")
    }

    pub fn copy_to_source(&mut self, input: &Utf8Path) -> Result<(), Error> {
        let content = std::fs::read_to_string(input).map_err(|err| Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Read,
            path: input.into(),
            err: Some(err.to_string()),
        })?;
        let to = input.file_name().expect("Extract file name");
        self.write_source(to, &content);
        Ok(())
    }

    pub fn write_out(&mut self, name: &str, content: &str) {
        let path = Project::out().join(name);
        self.fs
            .write(&path, content)
            .expect("Write a file in memory");
    }
}

pub fn show_gleam_error(err: Error) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    err.pretty(&mut buffer);
    buffer_writer
        .print(&buffer)
        .expect("Write warning to stderr");
}

pub fn get_module(modules: Vec<Module>, name: &str) -> Option<Module> {
    modules.into_iter().find(|m| m.name == name)
}

pub fn get_main_function(module: &Module) -> Result<ModuleFunction, Error> {
    module.ast.type_info.get_main_function(Target::JavaScript)
}

pub fn compile(project: &mut Project, repl: bool) -> Result<Vec<Module>, Error> {
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

fn to_error_stdio(err: std::io::Error) -> Error {
    Error::StandardIo {
        action: gleam_core::error::StandardIoAction::Read,
        err: Some(err.kind()),
    }
}

pub fn to_error_nonutf8_path(path: PathBuf) -> Error {
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
            .expect("Write warning to stderr");
    }
}

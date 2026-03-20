use camino::{Utf8Path, Utf8PathBuf};
use flate2::read::GzDecoder;
use gleam_core::{
    ast::{Definition, Function, UntypedDefinition, UntypedExpr},
    build::{
        Mode, Module, NullTelemetry, PackageCompiler, StaleTracker, Target,
        TargetCodegenConfiguration,
    },
    config::PackageConfig,
    error::{FileIoAction, FileKind},
    io::{memory::InMemoryFileSystem, FileSystemReader, FileSystemWriter},
    parse::parse_module,
    type_::{printer::Printer, Type},
    uid::UniqueIdGenerator,
    warning::{VectorWarningEmitterIO, WarningEmitter, WarningEmitterIO},
    Error, Warning,
};
use std::{
    collections::{HashSet, VecDeque},
    io::{Read, Write},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::SystemTime,
};
use tar::Archive;
use termcolor::{Color, ColorSpec, WriteColor};

#[cfg(not(target_arch = "wasm32"))]
use crate::GLEAM_STDLIB;
use crate::{error::{flush_buffer, stderr_buffer_writer}, GLEAM_STDLIB_BIGINT};

#[derive(Clone)]
pub struct Project {
    pub fs: InMemoryFileSystem,
}

fn stdlib() -> &'static [u8] {
    #[cfg(not(target_arch = "wasm32"))]
    if !gleam_core::javascript::is_bigint_enabled() {
        return GLEAM_STDLIB;
    }
    GLEAM_STDLIB_BIGINT
}

impl Default for Project {
    fn default() -> Project {
        #[cfg(target_arch = "wasm32")]
        gleam_core::javascript::set_bigint_enabled(true);

        let mut project = Project {
            fs: InMemoryFileSystem::new(),
        };

        extract_tar(&mut project.fs, stdlib(), Project::source()).expect("Extract stdlib");

        for path in crate::Sgleam::iter() {
            if let Some(content) = crate::Sgleam::get(&path) {
                if let Ok(content) = std::str::from_utf8(&content.data) {
                    project.write_source(&path, content);
                }
            }
        }

        project.write_out("prelude.mjs", gleam_core::javascript::prelude());
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

    pub fn copy_file_to_source(&mut self, input: &Utf8Path) -> Result<(), Error> {
        let content = std::fs::read_to_string(input).map_err(|err| Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Read,
            path: input.into(),
            err: Some(err.to_string()),
        })?;
        self.write_source(&input.as_str().replace('\\', "/"), &content);
        Ok(())
    }

    pub fn write_out(&mut self, name: &str, content: &str) {
        let path = Project::out().join(name);
        self.fs
            .write(&path, content)
            .expect("Write a file in memory");
    }

    #[allow(unused)]
    pub fn dump(&mut self) {
        for path in self.fs.files() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, self.fs.read_bytes(&path).unwrap()).unwrap();
        }
    }
}

pub fn get_module<'a>(modules: &'a [Module], name: &str) -> Option<&'a Module> {
    modules.iter().find(|m| m.name == name)
}

pub fn type_to_string(module: &Module, type_: &Type) -> String {
    Printer::new(&module.ast.names).print_type(type_).into()
}

pub fn fn_type_to_string(module: &Module, args: &[Arc<Type>], return_: Arc<Type>) -> String {
    type_to_string(
        module,
        &Type::Fn {
            arguments: args.into(),
            return_,
        },
    )
}

pub fn get_definition_src<'a>(def: &UntypedDefinition, src: &'a str) -> &'a str {
    let start = def.location().start as usize;
    let end = def.location().end as usize;
    let end = match def {
        Definition::TypeAlias(_) | Definition::Import(_) => end,
        Definition::CustomType(type_) => type_.end_position as usize,
        Definition::ModuleConstant(const_) => const_.value.location().end as usize,
        Definition::Function(f) => f.end_position as usize,
    };

    &src[start..end]
}

pub fn get_args_names(fun: &Function<(), UntypedExpr>) -> Vec<String> {
    fun.arguments
        .iter()
        .filter_map(|arg| arg.names.get_variable_name().map(String::from))
        .collect()
}

// TODO: move this function to Project
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
        .map(|out| out.modules)
        .into_result()
}

pub fn find_imports(paths: Vec<Utf8PathBuf>) -> Result<Vec<Utf8PathBuf>, gleam_core::Error> {
    let warning_emitter = WarningEmitter::new(Rc::new(VectorWarningEmitterIO::new()));
    let mut files: Vec<Utf8PathBuf> = vec![];
    let mut pending = VecDeque::from(paths);
    while let Some(path) = pending.pop_front() {
        if files.contains(&path) {
            continue;
        }

        files.push(path.clone());

        let src = std::fs::read_to_string(&path).map_err(|err| gleam_core::Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Read,
            path: path.clone(),
            err: Some(err.to_string()),
        })?;

        let parsed = parse_module(path.clone(), &src, &warning_emitter).map_err(|error| {
            gleam_core::Error::Parse {
                path,
                src: src.into(),
                error: error.into(),
            }
        })?;

        for definition in &parsed.module.definitions {
            match &definition.definition {
                gleam_core::ast::Definition::Import(import)
                    if import.module != "sgleam"
                        && !import.module.starts_with("sgleam/")
                        && !import.module.starts_with("gleam/") =>
                {
                    let mut path = Utf8PathBuf::new();
                    for p in import.module.split("/") {
                        path.push(p);
                    }
                    path.set_extension("gleam");
                    pending.push_back(path);
                }
                _ => continue,
            }
        }
    }
    Ok(files)
}

fn extract_tar(fs: &mut InMemoryFileSystem, data: &[u8], to: &Utf8Path) -> Result<(), Error> {
    let decoder = GzDecoder::new(data);
    let mut arch = Archive::new(decoder);
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
                    gleam_core::type_::Warning::Todo { .. }
                    | gleam_core::type_::Warning::UnreachableCodeAfterPanic { .. }
                    | gleam_core::type_::Warning::UnusedConstructor { .. }
                    | gleam_core::type_::Warning::UnusedImportedModule { .. }
                    | gleam_core::type_::Warning::UnusedImportedModuleAlias { .. }
                    | gleam_core::type_::Warning::UnusedImportedValue { .. }
                    | gleam_core::type_::Warning::RedundantAssertAssignment { .. }
                    // | gleam_core::type_::Warning::UnusedLiteral { .. }
                    | gleam_core::type_::Warning::UnusedPrivateFunction { .. }
                    | gleam_core::type_::Warning::UnusedPrivateModuleConstant { .. }
                    | gleam_core::type_::Warning::UnusedType { .. }
                    // | gleam_core::type_::Warning::UnusedValue { .. }
                    | gleam_core::type_::Warning::UnusedVariable { .. },
                ..
            } = warning
            {
                return;
            }
        }
        let buffer_writer = stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        warning.pretty(&mut buffer);
        flush_buffer(&buffer_writer, &buffer);
    }
}

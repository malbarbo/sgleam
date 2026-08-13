use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use ecow::EcoString;
use flate2::read::GzDecoder;
use gleam_core::{
    Error, Warning,
    ast::{Definition, SrcSpan, UntypedDefinition},
    build::{
        Mode, Module, NullTelemetry, PackageCompiler, StaleTracker, Target,
        TargetCodegenConfiguration,
    },
    config::PackageConfig,
    diagnostic::Diagnostic,
    error::{DefinedModuleOrigin, FileIoAction, FileKind},
    io::{FileSystemReader, FileSystemWriter, memory::InMemoryFileSystem},
    parse::parse_module,
    type_::{
        Type,
        printer::{Names, Printer},
    },
    uid::UniqueIdGenerator,
    warning::{VectorWarningEmitterIO, WarningEmitter, WarningEmitterIO},
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

use crate::{
    GLEAM_STDLIB,
    error::{flush_buffer, stderr_buffer_writer},
};

#[derive(Clone)]
pub struct Project {
    pub fs: InMemoryFileSystem,
}

impl Default for Project {
    fn default() -> Project {
        let mut project = Project {
            fs: InMemoryFileSystem::new(),
        };

        extract_tar(&mut project.fs, GLEAM_STDLIB, Project::source()).expect("Extract stdlib");

        for path in crate::Sgleam::iter() {
            if let Some(content) = crate::Sgleam::get(&path)
                && let Ok(content) = std::str::from_utf8(&content.data)
            {
                project.write_source(&path, content);
            }
        }

        project.write_out("prelude.mjs", gleam_core::javascript::PRELUDE);
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

    /// `name` is the module the content will be compiled as, hence a path under
    /// the source root: one that is not joins to itself, landing where nothing
    /// compiles it.
    pub fn write_source(&mut self, name: &str, content: &str) {
        assert!(
            is_module_path(name),
            "`{name}` is not under the source root"
        );
        let path = Project::source().join(name);
        self.fs
            .write(&path, content)
            .expect("Write a file in memory");
        self.fs
            .try_set_modification_time(&path, SystemTime::now())
            .expect("Set modification time of a file in memory")
    }

    /// The module the file was written as. A module is named after the path it
    /// sits at under the source root, so only the write knows the name.
    pub fn copy_file_to_source(&mut self, input: &Utf8Path) -> Result<EcoString, Error> {
        let content = std::fs::read_to_string(input).map_err(|err| Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Read,
            path: input.into(),
            err: Some(err.to_string()),
        })?;
        let path = input.as_str().replace('\\', "/");
        self.write_source(&path, &content);
        Ok(path.strip_suffix(".gleam").unwrap_or(&path).into())
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

    pub fn compile(&mut self, repl: bool) -> Result<Vec<Module>, Error> {
        self.compile_with_modules(
            Rc::new(ConsoleWarningEmitter::with_repl(repl)),
            &mut im::HashMap::new(),
            &mut im::HashMap::new(),
        )
    }

    pub fn compile_with_modules(
        &mut self,
        warnings: Rc<dyn WarningEmitterIO>,
        existing_modules: &mut im::HashMap<EcoString, gleam_core::type_::ModuleInterface>,
        defined_modules: &mut im::HashMap<EcoString, DefinedModuleOrigin>,
    ) -> Result<Vec<Module>, Error> {
        let config = PackageConfig {
            target: Target::JavaScript,
            ..Default::default()
        };

        let target = TargetCodegenConfiguration::JavaScript {
            emit_typescript_definitions: false,
            emit_source_maps: false,
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
            self.fs.clone(),
        );

        compiler.write_metadata = true;

        compiler
            .compile(
                &WarningEmitter::new(warnings),
                existing_modules,
                defined_modules,
                &mut StaleTracker::default(),
                &mut HashSet::new(),
                &NullTelemetry,
            )
            .map(|out| out.modules)
            .into_result()
    }
}

/// The path the user gave, from the one its copy sits at.
pub fn user_path(path: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(Project::source())
        .map_or_else(|_| path.to_path_buf(), Utf8Path::to_path_buf)
}

pub fn relocate_to_user_paths(diagnostic: &mut Diagnostic) {
    let Some(location) = &mut diagnostic.location else {
        return;
    };
    location.path = user_path(&location.path);
    for extra in &mut location.extra_labels {
        if let Some((_, path)) = &mut extra.src_info {
            *path = user_path(path);
        }
    }
}

/// Whether the path names the module it would be written as: relative, and
/// made of names alone.
pub fn is_module_path(path: &str) -> bool {
    !path.is_empty()
        && Utf8Path::new(path)
            .components()
            .all(|component| matches!(component, Utf8Component::Normal(_)))
}

pub fn get_module<'a>(modules: &'a [Module], name: &str) -> Option<&'a Module> {
    modules.iter().find(|m| m.name == name)
}

pub fn type_to_string(names: &Names, type_: &Type) -> String {
    Printer::new(names).print_type(type_).into()
}

pub fn fn_type_to_string(module: &Module, args: &[Arc<Type>], return_: Arc<Type>) -> String {
    type_to_string(
        &module.ast.names,
        &Type::Fn {
            arguments: args.into(),
            return_,
        },
    )
}

/// Whether the input kept the definition private, the one case that has no
/// `pub` written at its keyword — `@internal` requires one.
pub fn is_private(def: &UntypedDefinition) -> bool {
    match def {
        Definition::Function(f) => f.publicity.is_private(),
        Definition::TypeAlias(t) => t.publicity.is_private(),
        Definition::CustomType(t) => t.publicity.is_private(),
        Definition::ModuleConstant(c) => c.publicity.is_private(),
        Definition::Import(_) => false,
    }
}

/// What the definition takes of the input it was parsed from, from `start`,
/// where the item that produced it began: `location` starts at the keyword,
/// leaving the attributes above it out, and stops at the head of the ones that
/// have a body.
pub fn get_definition_span(def: &UntypedDefinition, start: u32) -> SrcSpan {
    let end = match def {
        Definition::TypeAlias(_) | Definition::Import(_) => def.location().end,
        Definition::CustomType(type_) => type_.end_position,
        Definition::ModuleConstant(const_) => const_.value.location().end,
        // `end_position` is the closing brace, which a function with no body
        // has none of: there it stops at the parameters, before the return
        // annotation an external function is required to write.
        Definition::Function(f) => f.end_position.max(
            f.return_annotation
                .as_ref()
                .map_or(0, |annotation| annotation.location().end),
        ),
    };

    SrcSpan::new(start, end)
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

/// Warnings about the scaffolding and not about what the user wrote. Every name
/// in scope reaches a generated module by import, so one the input does not use
/// is the rule there, and a module under two names is what a session does over
/// time.
///
/// Nothing else is filtered: a definition of an input is public, so it is never
/// reported unused, and what is left — a `todo`, an unreachable line, a variable
/// a function never reads — is the compiler teaching, which is the point of the
/// thing.
pub fn is_repl_noise(warning: &Warning) -> bool {
    matches!(
        warning,
        Warning::Type {
            warning: gleam_core::type_::Warning::ModuleImportedTwice { .. }
                | gleam_core::type_::Warning::UnusedImportedModule { .. }
                | gleam_core::type_::Warning::UnusedImportedModuleAlias { .. }
                | gleam_core::type_::Warning::UnusedImportedValue { .. }
                | gleam_core::type_::Warning::UnusedConstructor { imported: true, .. }
                | gleam_core::type_::Warning::UnusedType { imported: true, .. },
            ..
        }
    )
}

impl WarningEmitterIO for ConsoleWarningEmitter {
    fn emit_warning(&self, warning: Warning) {
        if self.repl && is_repl_noise(&warning) {
            return;
        }
        // This one writes the path into its hint, leaving no location to move.
        let warning = match warning {
            Warning::InvalidSource { path } => Warning::InvalidSource {
                path: user_path(&path),
            },
            warning => warning,
        };
        let mut diagnostic = warning.to_diagnostic();
        relocate_to_user_paths(&mut diagnostic);

        let buffer_writer = stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        diagnostic.write(&mut buffer);
        writeln!(buffer).expect("write newline after a warning");
        flush_buffer(&buffer_writer, &buffer);
    }
}

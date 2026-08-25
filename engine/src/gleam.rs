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
    io::{FileSystemWriter, memory::InMemoryFileSystem},
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

        for path in crate::SgleamLib::iter() {
            let file = crate::SgleamLib::get(&path).expect("Read an embedded sgleam file");
            let content = std::str::from_utf8(&file.data).expect("An embedded sgleam file is utf8");
            project.write_source(&path, content);
        }

        project.write_out("prelude.mjs", gleam_core::javascript::PRELUDE);
        project
    }
}

impl Project {
    fn root() -> &'static Utf8Path {
        "/".into()
    }

    pub fn source() -> &'static Utf8Path {
        "/src".into()
    }

    pub fn out() -> &'static Utf8Path {
        "/build".into()
    }

    /// The path the compiled JavaScript uses for a source file: relative to the
    /// project root, so `repl1.gleam` is `src/repl1.gleam`.
    pub fn source_path(name: &str) -> Utf8PathBuf {
        Project::source()
            .strip_prefix(Project::root())
            .expect("The source root is under the project root")
            .join(name)
    }

    fn prelude() -> &'static Utf8Path {
        "/build/prelude.mjs".into()
    }

    /// `name` is the path the module takes its name from, so it has to stay
    /// under the source root: an absolute name replaces the root outright, and
    /// a name with `..` lands outside, where nothing compiles it.
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

    /// Returns the name of the module the file becomes: its path, minus the
    /// `.gleam`.
    pub fn copy_file_to_source(&mut self, input: &Utf8Path) -> Result<EcoString, Error> {
        let content = std::fs::read_to_string(input).map_err(|err| Error::FileIo {
            kind: FileKind::File,
            action: FileIoAction::Read,
            path: input.into(),
            err: Some(err.to_string()),
        })?;
        // A module name always separates with `/`, which on Windows the path
        // does not.
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

/// A path as the user gave it: the source root is where sgleam put the file,
/// not where the user wrote it.
fn user_path(path: &Utf8Path) -> Utf8PathBuf {
    path.strip_prefix(Project::source())
        .unwrap_or(path)
        .to_path_buf()
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

/// Whether the path can name a module: a module takes its name from its path,
/// so the path has to be relative and made of plain names.
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

pub fn is_private(def: &UntypedDefinition) -> bool {
    match def {
        Definition::Function(f) => f.publicity.is_private(),
        Definition::TypeAlias(t) => t.publicity.is_private(),
        Definition::CustomType(t) => t.publicity.is_private(),
        Definition::ModuleConstant(c) => c.publicity.is_private(),
        Definition::Import(_) => false,
    }
}

/// How much of the input the definition covers, from `start`, where the item
/// began: `location` stops at the head of a definition that has a body.
pub fn get_definition_span(def: &UntypedDefinition, start: u32) -> SrcSpan {
    let end = match def {
        Definition::TypeAlias(_) | Definition::Import(_) => def.location().end,
        Definition::CustomType(type_) => type_.end_position,
        Definition::ModuleConstant(const_) => const_.value.location().end,
        // `end_position` is the closing brace, which a function with no body
        // has none of: there it stops at the parameters, before the return
        // annotation an external function must write.
        Definition::Function(f) => f.end_position.max(
            f.return_annotation
                .as_ref()
                .map_or(0, |annotation| annotation.location().end),
        ),
    };

    SrcSpan::new(start, end)
}

/// Whether sgleam brings the module itself — the prelude, the standard library,
/// the sgleam library. There is no file to look for.
fn is_builtin_module(module: &str) -> bool {
    matches!(module, "gleam" | "sgleam")
        || module.starts_with("gleam/")
        || module.starts_with("sgleam/")
}

/// The files of `paths` and of every module they import, directly or not.
pub fn find_imports(paths: Vec<Utf8PathBuf>) -> Result<Vec<Utf8PathBuf>, gleam_core::Error> {
    let warning_emitter = WarningEmitter::new(Rc::new(VectorWarningEmitterIO::new()));
    let mut files: Vec<Utf8PathBuf> = vec![];
    // A path an import led to is only a guess at where the module is written,
    // so it is allowed not to be there; one the caller gave is not.
    let mut pending: VecDeque<_> = paths.into_iter().map(|path| (path, true)).collect();
    while let Some((path, given)) = pending.pop_front() {
        if files.contains(&path) {
            continue;
        }

        let src = match std::fs::read_to_string(&path) {
            Ok(src) => src,
            // Nothing was found to compile the import against, which the
            // compiler says at the line that wrote it.
            Err(err) if !given && err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(gleam_core::Error::FileIo {
                    kind: FileKind::File,
                    action: FileIoAction::Read,
                    path,
                    err: Some(err.to_string()),
                });
            }
        };

        files.push(path.clone());

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
                    if !is_builtin_module(import.module.as_str()) =>
                {
                    let mut path = Utf8PathBuf::new();
                    for p in import.module.split("/") {
                        path.push(p);
                    }
                    path.set_extension("gleam");
                    pending.push_back((path, false));
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

fn to_error_nonutf8_path(path: PathBuf) -> Error {
    Error::NonUtf8Path { path }
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

/// Whether the warning is about the scaffolding the repl writes and not about
/// what the user wrote. The repl imports every name in scope into each module it
/// generates, so an unused import is the rule there, and a module already in
/// scope comes in twice when the input imports it under a new name.
///
/// Everything else stays: a `todo`, an unreachable line, a variable a function
/// never reads — the compiler teaches with those.
pub fn is_repl_noise(warning: &Warning) -> bool {
    match warning {
        Warning::Type { warning, .. } => matches!(
            **warning,
            gleam_core::type_::Warning::ModuleImportedTwice { .. }
                | gleam_core::type_::Warning::UnusedImportedModule { .. }
                | gleam_core::type_::Warning::UnusedImportedModuleAlias { .. }
                | gleam_core::type_::Warning::UnusedImportedValue { .. }
                | gleam_core::type_::Warning::UnusedConstructor { imported: true, .. }
                | gleam_core::type_::Warning::UnusedType { imported: true, .. }
        ),
        _ => false,
    }
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

use std::{
    collections::{BTreeMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use ecow::EcoString;
use gleam_core::{
    Error, Warning,
    ast::{
        AssignName, BitArrayOption, BitArraySize, Constant, Definition, Pattern, SrcSpan,
        Statement, UntypedConstant, UntypedPattern, UntypedStatement,
    },
    build::Module,
    diagnostic::Diagnostic,
    error::DefinedModuleOrigin,
    io::{FileSystemReader, FileSystemWriter},
    parse,
    type_::{ModuleInterface, printer::Names},
    warning::VectorWarningEmitterIO,
};
use indoc::formatdoc;

use crate::{
    engine::{Engine, MainFunction, ReplFile},
    gleam::{Project, get_definition_span, is_private, is_repl_noise, type_to_string},
    parser::{self, ReplItem},
    run::get_function,
    source::Source,
    swriteln,
};

pub const QUIT: &str = ":quit";
pub const TYPE: &str = ":type ";
pub const TIME: &str = ":time ";
pub const DEBUG: &str = ":debug";

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d or \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

/// Where a name in scope comes from: the module that defines it and the name
/// it has there. Every name reaches the input being run this way — an import,
/// a definition of an earlier input, or a `let` read back from its slot by the
/// companion module of the input that bound it.
#[derive(Clone)]
struct NameEntry {
    module: String,
    original: String,
    /// A `let`: a function of its companion module, read back by a binding the
    /// repl writes at the top of every body that names it. The one kind of
    /// value a const may not read.
    runtime: bool,
    /// What a const reads: a guard inlines it, and the inlined text names these
    /// where the const was defined. `None` for a name an import brought, whose
    /// const the repl never read — so it always comes in.
    reads: Option<Vec<String>>,
}

impl NameEntry {
    fn defined_in(module: &str, original: impl AsRef<str>) -> NameEntry {
        NameEntry {
            module: module.into(),
            original: original.as_ref().into(),
            runtime: false,
            reads: None,
        }
    }
}

/// What the import the input just wrote brings, so the repl can leave it out
/// of what it writes itself and put the input's own line in instead.
#[derive(Clone)]
struct OwnImport {
    input: Rc<str>,
    span: SrcSpan,
    path: String,
    alias: Option<String>,
    values: Vec<String>,
    types: Vec<String>,
}

/// What an input asks for. Anything that is none of the repl's own commands is
/// Gleam source to run.
enum Command<'a> {
    Quit,
    Debug,
    /// The type of an expression, which is not evaluated.
    Type(&'a str),
    /// An expression, and how long evaluating it takes.
    Time(&'a str),
    Source(&'a str),
}

impl<'a> Command<'a> {
    fn parse(input: &'a str) -> Command<'a> {
        let trimmed = input.trim();
        if trimmed == QUIT {
            Command::Quit
        } else if trimmed == DEBUG {
            Command::Debug
        } else if let Some(expr) = trimmed.strip_prefix(TYPE) {
            Command::Type(expr)
        } else if let Some(expr) = trimmed.strip_prefix(TIME) {
            Command::Time(expr)
        } else {
            // Not trimmed: the spans of the parsed input index this string.
            Command::Source(input)
        }
    }

    /// The Gleam of the input, which is what says whether it is finished. A
    /// command of the repl's own is finished when it is written.
    fn gleam(&self) -> Option<&'a str> {
        match self {
            Command::Quit | Command::Debug => None,
            Command::Type(src) | Command::Time(src) | Command::Source(src) => Some(src),
        }
    }
}

/// Whether the reader has to go on reading before this input can run. The
/// command is stripped first, or `:type case x {` would be asked about as
/// Gleam, which it is not.
pub fn is_incomplete(input: &str) -> bool {
    Command::parse(input)
        .gleam()
        .is_some_and(crate::parser::is_incomplete)
}

/// What a module is compiled for. One that only declares the scope, to check
/// an import, uses nothing in it — so there, and only there, a warning that
/// something is never used says nothing about what the user wrote.
#[derive(PartialEq, Eq)]
enum Purpose {
    Run,
    DeclareScope,
}

/// Which diagnostics reach the screen: the ones about the input alone, or
/// those too when nothing landed on it.
#[derive(PartialEq, Eq)]
enum Show {
    CopiesOnly,
    PreferCopies,
}

/// Why an input did not run.
enum InputError {
    /// What the compiler said about it.
    Compile(Error),
    /// A rule of the repl it broke, in the words the student reads.
    Repl(String),
}

impl From<Error> for InputError {
    fn from(error: Error) -> InputError {
        InputError::Compile(error)
    }
}

/// The input stopped, and whatever stopped it is already on the screen.
struct Bail;

/// A definition of the input being run that goes to a module of its own,
/// instead of being re-emitted into every module generated later.
struct Def {
    /// The name it binds as a type, when it is one.
    type_name: Option<String>,
    /// The names it binds as values: a function, a const, or the constructors
    /// of a type.
    value_names: Vec<String>,
    /// What it reads, when it is a const, a module of a qualified name included.
    reads: Vec<String>,
    /// What it takes of the input, the attributes above it included.
    span: SrcSpan,
    /// Where its keyword is, which is where `pub` goes: an attribute comes
    /// before it, and nothing may come between them.
    keyword: u32,
    /// Whether the input left it private, which is what asks for the `pub`.
    /// Read off the definition and not off the text in front of the keyword,
    /// which is `pub` under any spacing the lexer accepts, or none.
    private: bool,
    /// The body of a function, which the repl writes into.
    body: Option<Body>,
}

/// Where the bindings a function body reads go, and the names it already has.
struct Body {
    start: u32,
    params: Vec<String>,
}

/// What an input defines itself, which the module holding those definitions
/// imports from nowhere. One list per namespace, as the scope is kept: a type
/// and a value of the same name are two names, and defining one may not keep
/// the other from coming in.
#[derive(Default)]
struct Defined {
    types: Vec<String>,
    values: Vec<String>,
}

impl Defined {
    fn of(defs: &[Def]) -> Defined {
        Defined {
            types: defs
                .iter()
                .filter_map(|def| def.type_name.clone())
                .collect(),
            values: defs
                .iter()
                .flat_map(|def| def.value_names.clone())
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct Repl<E: Engine> {
    // One map per Gleam namespace, so a name in one cannot evict a name in
    // another: `import gleam/list`, `type list` and `fn list()` all coexist.
    //
    // BTreeMap (not HashMap) so the generated source lists imports, types and
    // functions in a stable, cross-run order — compiler diagnostics that
    // reference line numbers in it stay reproducible.
    /// `import gleam/int as i` → "i" → "gleam/int"
    modules: BTreeMap<String, String>,
    types: BTreeMap<String, NameEntry>,
    values: BTreeMap<String, NameEntry>,
    // The modules this session wrote that hold names: one per input that
    // defines, one per item that binds.
    own_modules: Vec<String>,
    // The module the values of the item being run are read back from, compiled
    // in the same pass as the module that computes them.
    pending_vals: Option<(String, Source)>,
    project: Project,
    existing_modules: im::HashMap<EcoString, ModuleInterface>,
    defined_modules: im::HashMap<EcoString, DefinedModuleOrigin>,
    engine: E,
    // The input being run and the item of it being run, which name the module
    // they are compiled into.
    input: usize,
    item: usize,
    var_index: usize,
    debug: bool,
    had_runtime_error: bool,
    // What `:time` reports.
    elapsed: std::time::Duration,
    // The modules written for the item being run, by the path they were
    // written to. Each carries which of its bytes are a copy of the input,
    // which is what moves a diagnostic back onto what the user wrote.
    generated: Vec<(camino::Utf8PathBuf, Source)>,
    // Every module the repl has written and the runtime has not been told of.
    // A module that ran nothing still raises later — a function of an input
    // that only defined — so what marks a file as the repl's is having
    // written it, not having run it.
    pending_files: Vec<ReplFile>,
    // The import the input just wrote, kept while the module that checks it is
    // built: it goes in as a copy of the input, and what it brought is left
    // out of what the repl writes, so the line is not written twice.
    own_import: Option<OwnImport>,
    // Internal function names with random suffix to avoid collisions with user code.
    repl_main: String,
    repl_print: String,
    repl_memo: String,
    repl_vals: String,
    repl_value: String,
}

#[repr(u32)]
pub enum ReplOutput {
    StdOut = 0,
    Error = 1,
    Quit = 2,
}

impl<E: Engine> Repl<E> {
    pub fn new(project: Project, user_module: Option<&Module>) -> Repl<E> {
        let fs = project.fs.clone();
        let suffix = format!(
            "{:08x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        );
        let mut repl = Repl {
            modules: BTreeMap::new(),
            types: BTreeMap::new(),
            values: BTreeMap::new(),
            own_modules: Vec::new(),
            pending_vals: None,
            project,
            existing_modules: im::HashMap::new(),
            defined_modules: im::HashMap::new(),
            engine: E::new(fs),
            input: 0,
            item: 0,
            var_index: 0,
            debug: false,
            had_runtime_error: false,
            elapsed: std::time::Duration::ZERO,
            generated: Vec::new(),
            pending_files: Vec::new(),
            own_import: None,
            repl_main: format!("repl_main_{suffix}"),
            repl_print: format!("repl_print_{suffix}"),
            repl_memo: format!("repl_memo_{suffix}"),
            repl_vals: format!("repl_vals_{suffix}"),
            repl_value: format!("repl_value_{suffix}"),
        };
        if let Some(module) = user_module {
            repl.seed_module(module);
        }
        // Initial compilation, so the module interfaces completion reads are
        // available before the first input.
        repl.skip_taken_names();
        let _ = repl.run_check();
        repl
    }

    /// Seeds the project module's public names one by one, instead of a single
    /// blanket import, so that later definitions can shadow them.
    fn seed_module(&mut self, module: &Module) {
        let path = module.name.to_string();
        let interface = &module.ast.type_info;

        self.modules
            .insert(short_name(&path).to_string(), path.clone());

        for type_ in interface.public_type_names() {
            self.types
                .insert(type_.to_string(), NameEntry::defined_in(&path, type_));
        }

        for value in interface.public_value_names() {
            self.values
                .insert(value.to_string(), NameEntry::defined_in(&path, value));
        }
    }

    fn names(&self) -> impl Iterator<Item = &str> {
        self.values
            .keys()
            .chain(self.types.keys())
            .chain(self.modules.keys())
            .map(String::as_str)
    }

    /// Returns all completion candidates: unqualified names, qualified
    /// module.member names, and does NOT include keywords/commands (those
    /// are added by the CLI).
    pub fn completions(&self) -> Vec<String> {
        let mut result: Vec<String> = self.names().map(String::from).collect();
        for (alias, path) in &self.modules {
            let Some(iface) = self.existing_modules.get(path.as_str()) else {
                continue;
            };
            let values = iface.values.iter().map(|(name, v)| (name, v.publicity));
            let types = iface.types.iter().map(|(name, t)| (name, t.publicity));
            for (member, publicity) in values.chain(types) {
                if publicity.is_importable() {
                    result.push(format!("{alias}.{member}"));
                }
            }
        }
        result.sort();
        result.dedup();
        result
    }

    pub fn run(&mut self, input: &str) -> ReplOutput {
        self.had_runtime_error = false;

        match Command::parse(input) {
            Command::Quit => ReplOutput::Quit,
            Command::Debug => {
                self.debug = !self.debug;
                println!("Debug mode {}.", if self.debug { "on" } else { "off" });
                ReplOutput::StdOut
            }
            Command::Type(expr) => self.run_input(|repl| repl.guarded(|r| r.run_type_cmd(expr))),
            Command::Time(expr) => self.run_input(|repl| repl.guarded(|r| r.run_time_cmd(expr))),
            Command::Source(src) => self.run_input(|repl| repl.run_source(src)),
        }
    }

    /// Numbers the input and reports how it ended.
    fn run_input(&mut self, run: impl FnOnce(&mut Self) -> Result<(), Bail>) -> ReplOutput {
        // A failed input still spends its number: a module name is never
        // reused, as the engine holds the module it loaded under it.
        self.input += 1;
        self.item = 0;
        self.skip_taken_names();

        if run(self).is_err() || self.had_runtime_error {
            ReplOutput::Error
        } else {
            ReplOutput::StdOut
        }
    }

    /// Runs one unit of the input — its definitions, or one of its items —
    /// undoing it if it does not go in: only a unit that never ran leaves
    /// nothing behind.
    fn guarded(
        &mut self,
        run: impl FnOnce(&mut Self) -> Result<(), InputError>,
    ) -> Result<(), Bail> {
        // The snapshot is cheap — engine and project use reference counting
        // internally (Rc), so only the maps are copied.
        let snapshot = (*self).clone();
        let Err(error) = run(self) else {
            return Ok(());
        };
        // Shown before the state goes back, as placing a diagnostic on the
        // input reads what the input was compiled from.
        match &error {
            InputError::Compile(error) => self.show_gleam_error(error),
            InputError::Repl(message) => println!("{message}"),
        }
        *self = snapshot;
        Err(Bail)
    }

    /// The definitions and the statements of an input, in the order it writes
    /// them. It stops at the first failure: what is below was written expecting
    /// what is above to have worked.
    fn run_source(&mut self, src: &str) -> Result<(), Bail> {
        let input: Rc<str> = src.into();
        let mut items = Vec::new();
        // The definitions go in first, together in a module of their own, so
        // they can reference each other and the items below only have to import
        // them — the one unit that goes in whole or not at all, which costs
        // nothing before anything has run.
        self.guarded(|repl| {
            items = parse(&input)?;
            if let Some(reason) = repl.const_refusal(&items) {
                return Err(InputError::Repl(reason));
            }
            let defs = defs(&items);
            if !defs.is_empty() {
                repl.run_defs(&input, &defs)?;
            }
            Ok(())
        })?;

        for item in items {
            self.item += 1;
            self.guarded(|repl| match item {
                // Everything but an import is already compiled and bound by
                // `run_defs`.
                ReplItem::ReplDefinition(targeted, start) => {
                    if let Definition::Import(import) = &targeted.definition {
                        let span = get_definition_span(&targeted.definition, start);
                        repl.run_import(import, &input, span)?;
                    }
                    Ok(())
                }
                ReplItem::ReplStatement(statement) => repl.run_statement(&input, statement),
            })?;
            // What an item that raised did stays: its output is on the screen
            // either way.
            if self.had_runtime_error {
                return Err(Bail);
            }
        }

        Ok(())
    }

    // --- Source generation ---

    /// What the input has in scope, as source: the externals, the modules in
    /// scope and the names taken from them, `skip` aside, which are the names
    /// the generated module defines itself. Every name, a saved value included,
    /// comes in by import — no annotation is written here, so nothing a later
    /// input redefines can change what this module reads.
    ///
    /// Only the names `code` mentions come in, so an input does not pay for the
    /// whole scope. `None` writes them all, which is what checks an import.
    fn build_source(&self, code: Option<&str>, skip: &Defined) -> Source {
        let mentioned = code.map(|code| self.with_inlined(mentioned(code)));
        let mut src = Source::new();
        src.write(&self.build_externals());
        // The input's own import goes in as the input wrote it, so a
        // diagnostic about it is about a copy and not about a line the repl
        // reconstructed to look like one.
        if let Some(own) = &self.own_import {
            src.copy(&own.input, own.span);
            src.write("\n");
        }
        for (name, path) in &self.modules {
            if self
                .own_import
                .as_ref()
                .is_some_and(|own| own.alias.as_deref() == Some(name) && &own.path == path)
            {
                continue;
            }
            if mentioned
                .as_ref()
                .is_some_and(|names| !names.contains(name.as_str()))
            {
                continue;
            }
            if short_name(path) == name {
                swriteln!(src, "import {path}");
            } else {
                swriteln!(src, "import {path} as {name}");
            }
        }
        // A module of this session comes in when the input names it: what a
        // redefinition took the plain name of is reached through it. Unless an
        // import already writes the line — under that name, or of that module.
        for module in &self.own_modules {
            if mentioned
                .as_ref()
                .is_some_and(|names| names.contains(module.as_str()))
                && !self
                    .modules
                    .iter()
                    .any(|(name, path)| name == module || path == module)
            {
                swriteln!(src, "import {module}");
            }
        }
        // A value an import brought always comes in: a guard inlines a const,
        // and the repl never read what that one names. Nothing inlines a type.
        //
        // What the input defines is left out per namespace: a type and a value
        // of the same name are two names, and defining one may not keep the
        // other from coming in.
        for (kind, entries, inlinable, skip) in [
            ("", &self.values, true, &skip.values),
            ("type ", &self.types, false, &skip.types),
        ] {
            for (name, entry) in entries {
                let NameEntry {
                    module, original, ..
                } = entry;
                let dropped = (!inlinable || entry.reads.is_some())
                    && mentioned
                        .as_ref()
                        .is_some_and(|names| !names.contains(name.as_str()));
                let own = self.own_import.as_ref().is_some_and(|own| {
                    &own.path == module
                        && if inlinable { &own.values } else { &own.types }.contains(name)
                });
                if skip.contains(name) || dropped || own {
                    continue;
                }
                if name == original {
                    swriteln!(src, "import {module}.{{{kind}{original}}} as _");
                } else {
                    swriteln!(src, "import {module}.{{{kind}{original} as {name}}} as _");
                }
            }
        }
        src
    }

    /// Closes `names` over what its consts read, which the inlined text names
    /// where the source that reads the const never did.
    fn with_inlined(&self, mut names: HashSet<EcoString>) -> HashSet<EcoString> {
        let mut queue: Vec<EcoString> = names.iter().cloned().collect();
        while let Some(name) = queue.pop() {
            let Some(entry) = self.values.get(name.as_str()) else {
                continue;
            };
            for read in entry.reads.iter().flatten() {
                let read: EcoString = read.into();
                if names.insert(read.clone()) {
                    queue.push(read);
                }
            }
        }
        names
    }

    /// The FFI the generated modules reach the engine through.
    fn build_externals(&self) -> String {
        let (memo, print) = (&self.repl_memo, &self.repl_print);
        formatdoc! {r#"
            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_memo")
            pub fn {memo}(index: Int, value: fn() -> a) -> a

            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print")
            pub fn {print}(value: a) -> a
        "#}
    }

    /// The bindings a body reads before what the user wrote: Gleam has no
    /// value at module level, so a `let` of an earlier item is a function of
    /// its companion module, and reading it back is what makes the name mean
    /// the value in the text that names it.
    ///
    /// At the first statement of a body the scope holds the module level names
    /// and the parameters, and nothing else — so leaving out the parameters
    /// and what this input defines is not a precaution, it is the whole of it.
    fn injections(&self, code: &str, defined: &[String], params: &[String]) -> String {
        let mentioned = mentioned(code);
        let mut src = String::new();
        for (name, entry) in &self.values {
            if entry.runtime
                && mentioned.contains(name.as_str())
                && !defined.contains(name)
                && !params.contains(name)
            {
                let _ = writeln!(src, "let {name} = {name}()");
            }
        }
        src
    }

    // --- Compilation helpers ---

    /// The definitions of an input take the plain name, as the user reads it
    /// back in the type of a value the input that redefined the name left
    /// behind.
    fn module_name(&self) -> String {
        match self.item {
            0 => format!("repl{}", self.input),
            item => format!("repl{}_{item}", self.input),
        }
    }

    /// Writes a module of this session, producing its file name.
    fn write_source(&mut self, module_name: &str, code: &str) -> String {
        let file = format!("{module_name}.gleam");
        if self.debug {
            let mut formatted = String::new();
            if gleam_core::format::pretty(
                &mut formatted,
                &code.into(),
                camino::Utf8Path::new(&file),
            )
            .is_ok()
            {
                println!("--- {file} ---\n{formatted}---");
            } else {
                println!("--- {file} ---\n{code}\n---");
            }
        }
        self.project.write_source(&file, code);
        file
    }

    /// Skips the numbers a module of the user's already goes by: `repl1.gleam`
    /// is a plausible file name, and the module written over it would be lost.
    fn skip_taken_names(&mut self) {
        while self.name_taken() {
            self.input += 1;
        }
    }

    fn name_taken(&self) -> bool {
        let name = format!("repl{}", self.input);
        let prefix = format!("{name}_");
        self.project.fs.files().iter().any(|path| {
            path.parent() == Some(Project::source())
                && path
                    .file_stem()
                    .is_some_and(|stem| stem == name || stem.starts_with(prefix.as_str()))
        })
    }

    fn compile(
        &mut self,
        module_name: &str,
        src: Source,
        purpose: Purpose,
    ) -> Result<Module, Error> {
        self.generated.clear();
        let mut files = vec![];
        // The module the values of this item are read back from goes in here,
        // and not in a pass of its own: one pass compiles both, so a `let`
        // costs what any other input costs.
        if let Some((name, vals)) = self.pending_vals.take() {
            files.push(self.write_source(&name, vals.as_str()));
            self.remember(&files[0], vals);
        }
        let file = self.write_source(module_name, src.as_str());
        self.remember(&file, src);
        files.push(file);
        let repl_files: Vec<_> = files.iter().map(|file| self.repl_file(file)).collect();
        self.pending_files.extend(repl_files);

        self.defined_modules.clear();
        // Collected instead of printed as they are emitted, so they can be
        // relocated like the errors.
        let warnings = VectorWarningEmitterIO::new();
        let result = self.project.compile_with_modules(
            Rc::new(warnings.clone()),
            &mut self.existing_modules,
            &mut self.defined_modules,
        );

        // Dropped as soon as they are compiled: what the next input needs of
        // them is the interface and the JavaScript, not the source.
        for file in files {
            self.project
                .fs
                .delete_file(&Project::source().join(file))
                .expect("To delete repl file");
        }

        // A warning that does not land on a copy of the input is about what
        // the repl wrote, and the user cannot act on it.
        self.show_diagnostics(
            warnings
                .take()
                .iter()
                .filter(|warning| !self.is_noise(warning, &purpose))
                .map(|warning| warning.to_diagnostic())
                .collect(),
            Show::CopiesOnly,
        );

        let mut modules = result?;

        if self.debug {
            let js_path = format!("/build/{module_name}.mjs");
            if let Ok(js) = self.project.fs.read(camino::Utf8Path::new(&js_path)) {
                println!("--- {module_name}.mjs ---\n{js}---");
            }
        }

        let pos = modules
            .iter()
            .position(|module| module.name == module_name)
            .expect("The repl module");

        Ok(modules.swap_remove(pos))
    }

    /// Keeps what a module was written from, so a diagnostic about it can be
    /// read against the input it copied.
    fn remember(&mut self, file: &str, src: Source) {
        self.generated.push((Project::source().join(file), src));
    }

    /// A module the repl wrote, for the runtime: the lines of the input it was
    /// copied from, which is how a place in it reaches the user's own text.
    fn repl_file(&self, file: &str) -> ReplFile {
        let path = Project::source().join(file);
        let lines = self
            .generated
            .iter()
            .find(|(generated, _)| *generated == path)
            .map(|(_, src)| src.input_lines())
            .unwrap_or_default();
        ReplFile {
            path: file.into(),
            lines,
        }
    }

    /// Compile source with a `repl_main` body appended.
    fn compile_main(&mut self, body: &Source) -> Result<Module, Error> {
        let mut code = Source::new();
        code.write(&format!("pub fn {}() {{\n", self.repl_main));
        code.write(&self.injections(body.as_str(), &[], &[]));
        code.append(body);
        code.write("\n}\n");
        let mut src = self.build_source(Some(code.as_str()), &Defined::default());
        src.append(&code);
        let module = self.module_name();
        self.compile(&module, src, Purpose::Run)
    }

    fn show_gleam_error(&self, err: &Error) {
        let mut err = err.clone();
        // A type in the message is printed in the names of the module that
        // failed, which need the scope over them just as `type_names` does.
        if let Error::Type { failed_modules, .. } = &mut err {
            for module in failed_modules.values_mut() {
                self.register_types(&mut module.names);
            }
        }
        self.show_diagnostics(err.to_diagnostics(), Show::PreferCopies);
    }

    /// The names a type is printed in: those of the module it was compiled in,
    /// which hold only what that module named, plus the scope over them.
    /// Registering takes the plain name from whatever had it, so it goes to the
    /// newest definition, as it does for the user.
    fn type_names(&self, module: &Module) -> Names {
        let mut names = module.ast.names.clone();
        self.register_types(&mut names);
        names
    }

    fn register_types(&self, names: &mut Names) {
        for (name, entry) in &self.types {
            names.named_type_in_scope(
                entry.module.as_str().into(),
                entry.original.as_str().into(),
                name.into(),
            );
        }
    }

    /// A warning the scaffolding causes rather than the input.
    fn is_noise(&self, warning: &Warning, purpose: &Purpose) -> bool {
        *purpose == Purpose::DeclareScope && is_repl_noise(warning)
    }

    /// Print diagnostics moved onto the input they are about.
    fn show_diagnostics(&self, diags: Vec<Diagnostic>, show: Show) {
        use std::io::Write as _;
        if diags.is_empty() {
            return;
        }

        let mut diags: Vec<_> = diags
            .into_iter()
            .map(|mut diag| {
                let moved = self.move_onto_input(&mut diag);
                (diag, moved)
            })
            .collect();

        // One that stayed put is about what the repl wrote. It is dropped
        // outright for a warning, and for an error only when another one says
        // the same thing about the input — an error the repl cannot place is
        // worth more on the screen than nothing at all.
        if show == Show::CopiesOnly || diags.iter().any(|(_, moved)| *moved) {
            diags.retain(|(_, moved)| *moved);
        }
        if diags.is_empty() {
            return;
        }

        let buffer_writer = crate::error::stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        for (diag, _) in &diags {
            diag.write(&mut buffer);
            writeln!(buffer).expect("write newline");
        }
        crate::error::flush_buffer(&buffer_writer, &buffer);
    }

    /// Move `diag` onto what the input it points into wrote, producing whether
    /// it pointed into one. A diagnostic about another module, or about what
    /// the repl wrote, stays where it is.
    fn move_onto_input(&self, diag: &mut Diagnostic) -> bool {
        let Some(loc) = &mut diag.location else {
            return false;
        };
        let Some((_, src)) = self.generated.iter().find(|(path, _)| *path == loc.path) else {
            return false;
        };
        let Some(located) = src.locate(loc.label.span) else {
            return false;
        };

        loc.src = located.input.as_ref().into();
        loc.path = "<repl>".into();
        loc.label.span = located.span;
        // An extra label points at another file, and is left alone, or at this
        // one, and is kept for as long as it says something about the input.
        loc.extra_labels.retain_mut(|extra| {
            if extra.src_info.is_some() {
                return true;
            }
            let Some(extra_located) = src.locate(extra.label.span) else {
                return false;
            };
            extra.label.span = extra_located.span;
            true
        });
        true
    }

    /// Compile and execute a `repl_main` body.
    fn compile_and_run(&mut self, body: &Source) -> Result<Module, Error> {
        let module = self.compile_main(body)?;
        self.run_repl_main(&module);
        Ok(module)
    }

    fn run_repl_main(&mut self, module: &Module) {
        let start = std::time::Instant::now();
        let result = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain {
                name: self.repl_main.clone(),
                files: std::mem::take(&mut self.pending_files),
            },
            false,
        );
        self.elapsed = start.elapsed();
        if let Err(err) = result {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        }
    }

    /// Compile without a `repl_main`, over the whole scope: what checks an
    /// import, the names it brought included.
    fn run_check(&mut self) -> Result<(), Error> {
        let (module, src) = (
            self.module_name(),
            self.build_source(None, &Defined::default()),
        );
        self.compile(&module, src, Purpose::DeclareScope)
            .map(|_| ())
    }

    /// Builds the module the values an item binds are read back from: one
    /// function per name, over the tuple the run memoized. It says nothing
    /// about the session but the module that computes the tuple, so a name of
    /// it can never clash with one the input took.
    fn queue_vals_module(&mut self, from: &str, names: &[String]) -> String {
        // The plain name when the input has no definitions to hold it, so a
        // value is reached the way a type and a function are: `repl1.x()`.
        let plain = format!("repl{}", self.input);
        let module = if self.existing_modules.contains_key(plain.as_str()) {
            format!("repl{}_{}_vals", self.input, self.item)
        } else {
            plain
        };
        let vals = &self.repl_vals;
        let mut src = Source::new();
        swriteln!(src, "import {from}.{{{vals}}} as _");
        for (index, name) in names.iter().enumerate() {
            swriteln!(src, "pub fn {name}() {{ {vals}().{} }}", index + 1);
        }
        self.pending_vals = Some((module.clone(), src));
        module
    }

    // --- Item handlers ---

    fn run_statement(
        &mut self,
        input: &Rc<str>,
        statement: UntypedStatement,
    ) -> Result<(), InputError> {
        let location = statement.location();

        match statement {
            Statement::Use(_) => Err(InputError::Repl(
                "use statements are not supported outside blocks.".into(),
            )),
            Statement::Expression(_) => self.run_expr(input, location),
            Statement::Assignment(a) => {
                let mut names = vec![];
                assignment_find_names(&a.pattern, &mut names);
                let value = a.value.location();
                if names.is_empty() {
                    self.run_expr(input, location)
                } else {
                    let pattern = SrcSpan::new(location.start, a.pattern.location().end);
                    let annotation = a.annotation.as_ref().map(|t| t.location());
                    // What a `let assert` writes after the value: `as "message"`.
                    let message =
                        (value.end < location.end).then(|| SrcSpan::new(value.end, location.end));
                    self.run_assignment(input, pattern, annotation, value, message, &names)
                }
            }
            Statement::Assert(_) => self.run_assert(input, location),
        }
    }

    fn run_expr(&mut self, input: &Rc<str>, expr: SrcSpan) -> Result<(), InputError> {
        let module = self.compile_expr(input, expr)?;
        self.run_repl_main(&module);
        Ok(())
    }

    fn run_assert(&mut self, input: &Rc<str>, code: SrcSpan) -> Result<(), InputError> {
        let mut body = Source::new();
        body.copy(input, code);
        self.compile_and_run(&body)?;
        Ok(())
    }

    /// Binds what the input's pattern binds. Gleam has no value at module
    /// level, so what the session keeps is a function that runs the input once
    /// and remembers the tuple it produced — the annotation the repl used to
    /// write is the type Gleam reads off the input's own text instead.
    ///
    /// The value takes a name of the repl's before the pattern reads it, so
    /// each half of the input is written once: the annotation is checked where
    /// the user wrote it, and so is the pattern.
    fn run_assignment(
        &mut self,
        input: &Rc<str>,
        pattern: SrcSpan,
        annotation: Option<SrcSpan>,
        value: SrcSpan,
        message: Option<SrcSpan>,
        names: &[String],
    ) -> Result<(), InputError> {
        let slot = self.var_index;
        let (memo, print) = (self.repl_memo.clone(), self.repl_print.clone());
        let (vals, val) = (self.repl_vals.clone(), self.repl_value.clone());

        // Only the value and the message of a `let assert` read what the
        // session bound; a pattern names types and binds, so what it mentions
        // asks for no binding of its own.
        let mut compute = Source::new();
        compute.write(&format!("let {val}"));
        if let Some(annotation) = annotation {
            compute.write(": ");
            compute.copy(input, annotation);
        }
        compute.write(" = ");
        compute.copy(input, value);

        let mut reads = compute.as_str().to_string();
        if let Some(span) = message {
            reads.push_str(&input[span.start as usize..span.end as usize]);
        }

        let mut bind = Source::new();
        bind.write(&self.injections(&reads, &[], &[]));
        bind.append(&compute);
        bind.write("\n");
        bind.copy(input, pattern);
        bind.write(&format!(" = {val}"));
        if let Some(span) = message {
            bind.copy(input, span);
        }
        bind.write(&format!("\n#({val}"));
        for name in names {
            bind.write(&format!(", {name}"));
        }
        bind.write(")");

        let mut code = Source::new();
        code.write(&format!("pub fn {vals}() {{\n{memo}({slot}, fn() {{\n"));
        code.append(&bind);
        code.write(&format!(
            "\n}})\n}}\n\npub fn {main}() {{\n{print}({vals}().0)\n}}\n",
            main = self.repl_main
        ));

        let module = self.module_name();
        let vals_module = self.queue_vals_module(&module, names);
        let mut src = self.build_source(Some(code.as_str()), &Defined::default());
        src.append(&code);

        // Drops what an input that failed after binding left behind: the
        // engine appends, so `has_var` below only means "the value ran" while
        // the two agree on how many values there are.
        self.engine.truncate_vars(self.var_index);
        let module = self.compile(&module, src, Purpose::Run)?;
        self.run_repl_main(&module);

        if !self.engine.has_var(self.var_index) {
            // The value raised before it was remembered, so nothing binds.
            return Ok(());
        }
        self.var_index += 1;

        for name in names {
            self.values.insert(
                name.clone(),
                NameEntry {
                    module: vals_module.clone(),
                    original: name.clone(),
                    runtime: true,
                    reads: Some(vec![]),
                },
            );
        }
        self.own_modules.push(vals_module);

        Ok(())
    }

    /// Compiles one expression, printed when it runs.
    fn compile_expr(&mut self, input: &Rc<str>, expr: SrcSpan) -> Result<Module, Error> {
        let mut body = Source::new();
        body.write(&format!("{}({{\n", self.repl_print));
        body.copy(input, expr);
        body.write("\n})");
        self.compile_main(&body)
    }

    /// The one statement `:type` and `:time` take.
    fn command_statement(cmd: &str, input: &str) -> Result<UntypedStatement, InputError> {
        let refuse = |reason: &str| InputError::Repl(format!("{cmd}command {reason}"));
        let mut items = parse(input)?;
        if items.len() != 1 {
            return Err(refuse("expects exactly one expression."));
        }
        match items.swap_remove(0) {
            ReplItem::ReplStatement(statement) => Ok(statement),
            ReplItem::ReplDefinition(..) => Err(refuse("cannot be used with definitions.")),
        }
    }

    fn run_type_cmd(&mut self, expr: &str) -> Result<(), InputError> {
        // A command is an item of the input, not its definitions: item 0 names
        // the module a `let` of an input with none is read back from.
        self.item += 1;
        Self::command_statement(TYPE, expr)?;
        let input: Rc<str> = expr.into();
        let module = self.compile_expr(&input, SrcSpan::new(0, expr.len() as u32))?;
        let main = get_function(&module, &self.repl_main).expect("repl main function");
        println!(
            "{}",
            type_to_string(&self.type_names(&module), &main.return_type)
        );
        Ok(())
    }

    /// Takes a statement, not an expression: a `let` is one, and timing it
    /// binds like anywhere else.
    fn run_time_cmd(&mut self, expr: &str) -> Result<(), InputError> {
        self.item += 1;
        let statement = Self::command_statement(TIME, expr)?;
        self.run_statement(&expr.into(), statement)?;
        if !self.had_runtime_error {
            println!("Time: {}", format_duration(self.elapsed));
        }
        Ok(())
    }

    fn run_import(
        &mut self,
        import: &gleam_core::ast::Import<()>,
        input: &Rc<str>,
        span: SrcSpan,
    ) -> Result<(), Error> {
        let module = import.module.to_string();
        let mut own = OwnImport {
            input: input.clone(),
            span,
            path: module.clone(),
            alias: None,
            values: vec![],
            types: vec![],
        };

        // Handle module alias / short name.
        match &import.as_name {
            Some((gleam_core::ast::AssignName::Variable(name), _)) => {
                self.modules.insert(name.to_string(), module.clone());
                own.alias = Some(name.to_string());
            }
            // `as _` brings in the unqualified names only.
            Some((gleam_core::ast::AssignName::Discard(_), _)) => {}
            None => {
                self.modules
                    .insert(short_name(&module).to_string(), module.clone());
                own.alias = Some(short_name(&module).to_string());
            }
        }

        // Handle unqualified values
        for uv in &import.unqualified_values {
            let effective = uv
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| uv.name.to_string());
            own.values.push(effective.clone());
            self.values
                .insert(effective, NameEntry::defined_in(&module, &uv.name));
        }

        // Handle unqualified types
        for ut in &import.unqualified_types {
            let effective = ut
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| ut.name.to_string());
            own.types.push(effective.clone());
            self.types
                .insert(effective, NameEntry::defined_in(&module, &ut.name));
        }

        self.own_import = Some(own);
        let result = self.run_check();
        self.own_import = None;
        result
    }

    /// The reason a const of the input cannot be accepted, when there is one.
    /// What makes it a const is otherwise already checked: the parser accepts
    /// only a constant expression, and the one thing the parser cannot know is
    /// that a name it reads is a `let`, out of reach from module level as it
    /// would be in a source file.
    fn const_refusal(&self, items: &[ReplItem]) -> Option<String> {
        let (mut read, mut qualified) = (vec![], vec![]);
        for item in items {
            if let ReplItem::ReplDefinition(targeted, _) = item
                && let Definition::ModuleConstant(c) = &targeted.definition
            {
                constant_find_names(&c.value, &mut read, &mut qualified);
            }
        }
        let var = read
            .iter()
            .find(|name| self.values.get(*name).is_some_and(|entry| entry.runtime))?;
        Some(format!(
            "`{var}` is a variable, not a constant. A constant can only use \
             literals, other constants and functions."
        ))
    }

    /// Compiles the definitions of the input into a module of its own, kept for
    /// the rest of the session, and binds each name to that module. A later
    /// input imports the name instead of defining it again, so a redefinition
    /// leaves what was built on the old one untouched.
    fn run_defs(&mut self, input: &Rc<str>, defs: &[Def]) -> Result<(), Error> {
        let defined = Defined::of(defs);

        let mut code = Source::new();
        for def in defs {
            let text = &input[def.keyword as usize..def.span.end as usize];
            code.copy(input, SrcSpan::new(def.span.start, def.keyword));
            // Auto-pub, as the module it lands in is not the one that reads it.
            if def.private {
                code.write("pub ");
            }
            match &def.body {
                // The bindings a body reads go in ahead of what the user wrote,
                // splitting the definition in two copies of the input.
                Some(body) => {
                    code.copy(input, SrcSpan::new(def.keyword, body.start));
                    code.write(&self.injections(text, &defined.values, &body.params));
                    code.copy(input, SrcSpan::new(body.start, def.span.end));
                }
                None => code.copy(input, SrcSpan::new(def.keyword, def.span.end)),
            }
            code.write("\n");
        }
        let mut src = self.build_source(Some(code.as_str()), &defined);
        src.append(&code);

        let module = self.module_name();
        self.compile(&module, src, Purpose::Run)?;

        for def in defs {
            if let Some(name) = &def.type_name {
                self.types
                    .insert(name.clone(), NameEntry::defined_in(&module, name));
            }
            for name in &def.value_names {
                self.values.insert(
                    name.clone(),
                    NameEntry {
                        reads: Some(def.reads.clone()),
                        ..NameEntry::defined_in(&module, name)
                    },
                );
            }
        }
        self.own_modules.push(module);
        Ok(())
    }
}

/// The names the source mentions, as the lexer reads them: a label and a local
/// count too. Over-approximate on purpose — an import too many is noise, an
/// import missing is an error.
fn mentioned(code: &str) -> HashSet<EcoString> {
    parse::lexer::make_tokenizer(code)
        .filter_map(|token| match token {
            Ok((
                _,
                parse::token::Token::Name { name } | parse::token::Token::UpName { name },
                _,
            )) => Some(name),
            _ => None,
        })
        .collect()
}

/// The items of an input. Parsed on its own, so the error already points at it.
fn parse(input: &str) -> Result<Vec<ReplItem>, Error> {
    parser::parse_repl(input).map_err(|error| Error::Parse {
        path: "<repl>".into(),
        src: input.into(),
        error: error.into(),
    })
}

fn format_duration(elapsed: std::time::Duration) -> String {
    if elapsed.as_secs() > 0 {
        format!("{:.2} s", elapsed.as_secs_f64())
    } else if elapsed.as_millis() > 0 {
        format!("{} ms", elapsed.as_millis())
    } else if elapsed.as_micros() > 0 {
        format!("{} µs", elapsed.as_micros())
    } else {
        format!("{} ns", elapsed.as_nanos())
    }
}

/// The types, the functions and the consts the input defines, in the order it
/// defines them.
fn defs(items: &[ReplItem]) -> Vec<Def> {
    let mut defs = vec![];
    for item in items {
        let ReplItem::ReplDefinition(targeted, start) = item else {
            continue;
        };
        let mut reads = vec![];
        let mut body = None;
        let (type_name, value_names) = match &targeted.definition {
            // The constructors of an opaque type stay in the module that
            // defines it, which is no longer the one the next input reads.
            Definition::CustomType(t) if t.opaque => (Some(t.name.to_string()), vec![]),
            Definition::CustomType(t) => (
                Some(t.name.to_string()),
                t.constructors.iter().map(|c| c.name.to_string()).collect(),
            ),
            Definition::TypeAlias(t) => (Some(t.alias.to_string()), vec![]),
            Definition::Function(f) => {
                let name = f.name.clone().expect("A function must have a name").1;
                // An external function has no body, and so nothing to read the
                // session from.
                body = f.body_start.map(|brace| Body {
                    start: brace + 1,
                    params: f
                        .arguments
                        .iter()
                        .filter_map(|argument| argument.names.get_variable_name())
                        .map(EcoString::to_string)
                        .collect(),
                });
                (None, vec![name.to_string()])
            }
            Definition::ModuleConstant(c) => {
                let mut modules = vec![];
                constant_find_names(&c.value, &mut reads, &mut modules);
                reads.append(&mut modules);
                (None, vec![c.name.to_string()])
            }
            Definition::Import(_) => continue,
        };
        defs.push(Def {
            type_name,
            value_names,
            reads,
            span: get_definition_span(&targeted.definition, *start),
            keyword: targeted.definition.location().start,
            private: is_private(&targeted.definition),
            body,
        });
    }
    defs
}

fn assignment_find_names(pattern: &UntypedPattern, names: &mut Vec<String>) {
    match pattern {
        Pattern::Int { .. }
        | Pattern::Float { .. }
        | Pattern::String { .. }
        | Pattern::Discard { .. }
        | Pattern::Invalid { .. } => {}
        Pattern::Variable { name, .. } => names.push(name.into()),
        Pattern::StringPrefix {
            left_side_assignment,
            right_side_assignment,
            ..
        } => {
            if let Some((name, _)) = left_side_assignment {
                names.push(name.into());
            }
            if let AssignName::Variable(name) = right_side_assignment {
                names.push(name.into());
            }
        }
        Pattern::Assign { name, pattern, .. } => {
            names.push(name.into());
            assignment_find_names(pattern, names);
        }
        Pattern::List { elements, tail, .. } => {
            for element in elements {
                assignment_find_names(element, names);
            }
            if let Some(tail) = tail {
                assignment_find_names(&tail.pattern, names);
            }
        }
        Pattern::Constructor { arguments, .. } => {
            for argument in arguments {
                assignment_find_names(&argument.value, names);
            }
        }
        Pattern::Tuple { elements, .. } => {
            for element in elements {
                assignment_find_names(element, names);
            }
        }
        Pattern::BitArray { segments, .. } => {
            for segment in segments {
                assignment_find_names(&segment.value, names);
            }
        }
        Pattern::BitArraySize(bit_array_size) => bit_array_size_find_names(bit_array_size, names),
    }
}

/// Collects what a constant reads, as the text inlined at a guard names it: the
/// module level names, a constructor included, and the aliases of the qualified
/// ones, which go to `modules` — only a plain name can be a `let`.
fn constant_find_names(
    constant: &UntypedConstant,
    names: &mut Vec<String>,
    modules: &mut Vec<String>,
) {
    fn read(
        module: &Option<(EcoString, SrcSpan)>,
        name: &EcoString,
        names: &mut Vec<String>,
        modules: &mut Vec<String>,
    ) {
        match module {
            Some((alias, _)) => modules.push(alias.into()),
            None => names.push(name.into()),
        }
    }
    match constant {
        Constant::Int { .. }
        | Constant::Float { .. }
        | Constant::String { .. }
        | Constant::Invalid { .. } => {}
        Constant::Var { module, name, .. } => read(module, name, names, modules),
        Constant::Tuple { elements, .. } | Constant::List { elements, .. } => {
            for element in elements {
                constant_find_names(element, names, modules);
            }
        }
        Constant::Record {
            module,
            name,
            arguments,
            ..
        } => {
            read(module, name, names, modules);
            for argument in arguments {
                constant_find_names(&argument.value, names, modules);
            }
        }
        // Expanded into a `Record`, so the generated code names the constructor.
        Constant::RecordUpdate {
            module,
            name,
            record,
            arguments,
            ..
        } => {
            read(module, name, names, modules);
            constant_find_names(&record.base, names, modules);
            for argument in arguments {
                constant_find_names(&argument.value, names, modules);
            }
        }
        Constant::BitArray { segments, .. } => {
            for segment in segments {
                constant_find_names(&segment.value, names, modules);
                for option in &segment.options {
                    if let BitArrayOption::Size { value, .. } = option {
                        constant_find_names(value, names, modules);
                    }
                }
            }
        }
        Constant::StringConcatenation { left, right, .. } => {
            constant_find_names(left, names, modules);
            constant_find_names(right, names, modules);
        }
    }
}

fn bit_array_size_find_names(bit_array_size: &BitArraySize<()>, names: &mut Vec<String>) {
    match bit_array_size {
        BitArraySize::Int { .. } => {}
        BitArraySize::Variable { name, .. } => names.push(name.into()),
        BitArraySize::Block { inner, .. } => bit_array_size_find_names(inner, names),
        BitArraySize::BinaryOperator { left, right, .. } => {
            bit_array_size_find_names(left, names);
            bit_array_size_find_names(right, names);
        }
    }
}

fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

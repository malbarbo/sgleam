use std::{fmt::Write, rc::Rc};

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
    type_::{ModuleInterface, printer::Names},
    warning::VectorWarningEmitterIO,
};
use indoc::formatdoc;

use crate::{
    engine::{Engine, MainFunction, ReplFile},
    error::SgleamError,
    gleam::{
        Project, get_definition_span, is_private, is_repl_noise, relocate_to_user_paths,
        type_to_string,
    },
    parser::{self, ReplItem},
    run::get_function,
    scope::{Defined, NameEntry, Origin, Scope},
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

/// What an input asks for. Anything that is none of the repl's own commands is
/// Gleam source to run.
enum Command<'a> {
    Quit,
    Debug,
    /// The type of an expression, which is not evaluated.
    Type(&'a str),
    /// An expression, and how long evaluating it takes.
    Time(&'a str),
    /// An input that starts like a command but is none: `:typ x`, a bare
    /// `:type`. No Gleam starts with `:`, so the parser's complaint about one
    /// would only mislead.
    Unknown(&'a str),
    Gleam(&'a str),
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
        } else if trimmed.starts_with(':') {
            Command::Unknown(trimmed)
        } else {
            // Not trimmed: the spans of the parsed input index this string.
            Command::Gleam(input)
        }
    }

    /// The Gleam of the input, which is what says whether it is finished. A
    /// command of the repl's own always is.
    fn gleam(&self) -> Option<&'a str> {
        match self {
            Command::Quit | Command::Debug | Command::Unknown(_) => None,
            Command::Type(src) | Command::Time(src) | Command::Gleam(src) => Some(src),
        }
    }
}

/// What is wrong with an input that starts like a command but is none.
fn unknown_command(input: &str) -> String {
    let cmd = input.split_whitespace().next().unwrap_or(input);
    if cmd == QUIT || cmd == DEBUG {
        format!("The {cmd} command takes nothing after it.")
    } else if cmd == TYPE.trim_end() || cmd == TIME.trim_end() {
        format!("The {cmd} command takes an expression: `{cmd} 1 + 1`.")
    } else {
        format!(
            "Unknown command {cmd}.\nThe commands are {QUIT}, {DEBUG}, \
             {TYPE}<expr> and {TIME}<expr>."
        )
    }
}

/// Whether the reader has to go on reading before this input can run. The
/// command is stripped first, or `:type case x {` would be read as Gleam.
pub fn is_incomplete(input: &str) -> bool {
    Command::parse(input)
        .gleam()
        .is_some_and(crate::parser::is_incomplete)
}

/// What the prompt does with the input as it stands: `-1` runs it, and
/// anything else is how far in the next line starts, in spaces.
///
/// This is the whole of the question a reader asks on Enter, and both readers
/// ask it here -- the one in the terminal and the one the browser calls
/// through `repl_ready` (see SimpleCode's ENGINE.md).
///
/// An input with nothing open ends at a blank line, finished or not. That is
/// the only way out of one that will not close: an open bracket can be typed
/// shut, but `let x =` has none to type, and without this the line can only be
/// erased -- while the error the engine gives for it is the answer the user is
/// after. With a bracket open the rule would cost more than it gives, taking
/// the blank line between two statements of a function for the end of it.
pub fn ready_state(input: &str) -> i32 {
    if !is_incomplete(input) {
        return -1;
    }
    // Over the Gleam of the input, the way is_incomplete reads it: the command
    // in front of it is none of the brackets the next line lines up with.
    let depth = Command::parse(input)
        .gleam()
        .map_or(0, crate::parser::nesting_depth);
    if depth == 0
        && let Some((_, last)) = input.rsplit_once('\n')
        && last.trim().is_empty()
    {
        return -1;
    }
    (depth * INDENT) as i32
}

/// What one level of indentation is worth, in spaces.
const INDENT: usize = 2;

/// What a module is compiled for. Only one compiled to run is queued for the
/// runtime: one that answers `:type` is never called, and one that only
/// declares the scope, to check an import, is not even referenced. The latter
/// also uses nothing of the scope, so an unused-import warning there is
/// vacuous and dropped.
#[derive(PartialEq, Eq)]
enum Purpose {
    Run,
    Type,
    DeclareScope,
}

/// Which diagnostics reach the screen, after each is moved onto the copy of
/// the input it points into.
#[derive(PartialEq, Eq)]
enum Show {
    /// Only the ones that landed on the input: one that did not is about text
    /// the repl wrote, which the user cannot act on.
    OnInputOnly,
    /// The rest only when none landed on the input: an error with no place
    /// still says something.
    PreferOnInput,
}

/// Why an input did not run.
enum InputError {
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
    type_name: Option<String>,
    /// A function, a const, or the constructors of a type.
    value_names: Vec<String>,
    /// What it reads, when it is a const, a module of a qualified name included.
    reads: Vec<String>,
    /// What it takes of the input, the attributes above it included.
    span: SrcSpan,
    /// Where `pub` goes: after the attributes, and nothing may come between.
    keyword: u32,
    /// Whether the input left it private, which is what asks for the `pub`.
    /// Read off the definition: the text before the keyword is `pub` under any
    /// spacing the lexer accepts, or none.
    private: bool,
    /// The body of a function, which the repl writes into.
    body: Option<Body>,
}

/// Where the bindings a function body reads go, and the names it already has.
struct Body {
    start: u32,
    params: Vec<String>,
}

/// What the definitions bring, which their own module must not import.
fn defined_by(defs: &[Def]) -> Defined {
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

#[derive(Clone)]
pub struct Repl<E: Engine> {
    /// The names the session has in scope, and the modules that hold them.
    scope: Scope,
    /// The module the values of the item being run are read back from,
    /// compiled in the same pass as the module that computes them.
    pending_vals: Option<(String, Source)>,
    project: Project,
    existing_modules: im::HashMap<EcoString, ModuleInterface>,
    defined_modules: im::HashMap<EcoString, DefinedModuleOrigin>,
    engine: E,
    // The input and the item being run, which name the module they compile into.
    input_number: usize,
    item_number: usize,
    debug: bool,
    had_runtime_error: bool,
    /// What `:time` reports.
    elapsed: std::time::Duration,
    /// The modules written for the item being run, by the path they were
    /// written to, each keeping which of its bytes are a copy of the input.
    generated: Vec<(camino::Utf8PathBuf, Source)>,
    /// Every module the repl wrote to run that the runtime has not been told
    /// of. One that ran nothing still raises later, from a function it defined.
    pending_files: Vec<ReplFile>,
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
    pub fn new(project: Project, user_module: Option<&Module>) -> Result<Repl<E>, SgleamError> {
        let fs = project.fs.clone();
        let suffix = format!(
            "{:08x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        );
        let mut repl = Repl {
            scope: Scope::default(),
            pending_vals: None,
            project,
            existing_modules: im::HashMap::new(),
            defined_modules: im::HashMap::new(),
            engine: E::new(fs)?,
            input_number: 0,
            item_number: 0,
            debug: false,
            had_runtime_error: false,
            elapsed: std::time::Duration::ZERO,
            generated: Vec::new(),
            pending_files: Vec::new(),
            repl_main: format!("repl_main_{suffix}"),
            repl_print: format!("repl_print_{suffix}"),
            repl_memo: format!("repl_memo_{suffix}"),
            repl_vals: format!("repl_vals_{suffix}"),
            repl_value: format!("repl_value_{suffix}"),
        };
        if let Some(module) = user_module {
            repl.scope.seed_module(module);
        }
        // Compiled once here, so completion has the module interfaces to read.
        repl.skip_taken_names();
        if let Err(error) = repl.run_check() {
            repl.show_gleam_error(&error);
        }
        Ok(repl)
    }

    /// The completion candidates: every name in scope, and the public members
    /// of the imported modules, qualified. Keywords and commands are the CLI's
    /// to add.
    pub fn completions(&self) -> Vec<String> {
        let mut result: Vec<String> = self.scope.names().map(String::from).collect();
        for (alias, path) in &self.scope.modules {
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
            Command::Unknown(input) => {
                println!("{}", unknown_command(input));
                ReplOutput::Error
            }
            Command::Gleam(src) => self.run_input(|repl| repl.run_source(src)),
        }
    }

    fn run_input(&mut self, run: impl FnOnce(&mut Self) -> Result<(), Bail>) -> ReplOutput {
        // A failed input still spends its number: a module name is never
        // reused, as the engine holds the module it loaded under it.
        self.input_number += 1;
        self.item_number = 0;
        self.skip_taken_names();

        if run(self).is_err() || self.had_runtime_error {
            ReplOutput::Error
        } else {
            ReplOutput::StdOut
        }
    }

    /// Runs one unit of the input — its definitions, or one of its items —
    /// undoing it if it does not go in. Only a unit that never ran leaves
    /// nothing behind.
    fn guarded(
        &mut self,
        run: impl FnOnce(&mut Self) -> Result<(), InputError>,
    ) -> Result<(), Bail> {
        // The snapshot is cheap — engine and project use reference counting
        // internally (Rc), so only the maps are copied. The sharing also means
        // they are not rolled back: a module the engine loaded stays loaded,
        // which is fine because nothing of the session names it anymore.
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
        self.guarded(|repl| {
            items = parse(&input)?;
            if let Some(reason) = repl.const_refusal(&items) {
                return Err(InputError::Repl(reason));
            }
            Ok(())
        })?;

        // The imports go in ahead of everything else: a definition is compiled
        // against the scope, and what its own input imported is part of it.
        for (import, span) in imports(&items) {
            self.item_number += 1;
            self.guarded(|repl| {
                repl.run_import(import, &input, span)
                    .map_err(InputError::from)
            })?;
        }

        // The definitions go in next, in a module of their own, so they can
        // reference each other. All or nothing, which costs nothing: no item
        // has run yet.
        let defs = defs(&items);
        if !defs.is_empty() {
            self.guarded(|repl| repl.run_defs(&input, &defs).map_err(InputError::from))?;
        }

        for item in items {
            // Everything but a statement is already in by now.
            let ReplItem::ReplStatement(statement) = item else {
                continue;
            };
            self.item_number += 1;
            self.guarded(|repl| repl.run_statement(&input, statement))?;
            // What an item that raised did stays: its output is on the screen.
            if self.had_runtime_error {
                return Err(Bail);
            }
        }

        Ok(())
    }

    // --- Source generation ---

    /// What the input has in scope, as source: the externals, and the
    /// imports the scope writes for what `code` mentions.
    fn build_source(&self, code: Option<&str>, skip: &Defined) -> Source {
        let mut src = Source::new();
        src.write(&self.build_externals());
        self.scope.write_imports(&mut src, code, skip);
        src
    }

    /// The FFI the generated modules reach the engine through.
    fn build_externals(&self) -> String {
        let (memo, print) = (&self.repl_memo, &self.repl_print);
        formatdoc! {r#"
            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_memo")
            pub fn {memo}(key: String, value: fn() -> a) -> a

            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print")
            pub fn {print}(value: a) -> a
        "#}
    }

    // --- Compilation helpers ---

    fn module_name(&self) -> String {
        match self.item_number {
            0 => format!("repl{}", self.input_number),
            item => format!("repl{}_{item}", self.input_number),
        }
    }

    /// The definitions of an input take the plain name, as it is what the user
    /// reads back in the type of a value a later redefinition left behind. Not
    /// `module_name`: the imports are items, and they went first.
    fn defs_module_name(&self) -> String {
        format!("repl{}", self.input_number)
    }

    fn write_source(&mut self, module_name: &str, code: &str) -> String {
        let file = format!("{module_name}.gleam");
        if self.debug {
            let mut formatted = String::new();
            if gleam_format::pretty(&mut formatted, &code.into(), camino::Utf8Path::new(&file))
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
            self.input_number += 1;
        }
    }

    fn name_taken(&self) -> bool {
        let name = format!("repl{}", self.input_number);
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
        // and not in a pass of its own, so a `let` costs one compilation.
        if let Some((vals_module, vals)) = self.pending_vals.take() {
            let file = self.write_source(&vals_module, vals.as_str());
            self.generated.push((Project::source().join(&file), vals));
            files.push(file);
        }
        let file = self.write_source(module_name, src.as_str());
        self.generated.push((Project::source().join(&file), src));
        files.push(file);
        // Only a module compiled to run: nothing ever reaches a place in the
        // others, and loading them would cost the next run a module apiece.
        if purpose == Purpose::Run {
            let repl_files: Vec<_> = files.iter().map(|file| self.repl_file(file)).collect();
            self.pending_files.extend(repl_files);
        }

        self.defined_modules.clear();
        // Collected, not printed as emitted, so they are relocated like errors.
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

        self.show_diagnostics(
            warnings
                .take()
                .iter()
                .filter(|warning| !self.is_noise(warning, &purpose))
                .map(|warning| warning.to_diagnostic())
                .collect(),
            Show::OnInputOnly,
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

    /// A module the repl wrote, for the runtime: the input lines it came from.
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

    fn compile_main(&mut self, body: &Source, purpose: Purpose) -> Result<Module, Error> {
        let mut code = Source::new();
        code.write(&format!("pub fn {}() {{\n", self.repl_main));
        code.write(&self.scope.injections(body.as_str(), &[], &[]));
        code.append(body);
        code.write("\n}\n");
        let mut src = self.build_source(Some(code.as_str()), &Defined::default());
        src.append(&code);
        let module = self.module_name();
        self.compile(&module, src, purpose)
    }

    fn show_gleam_error(&self, err: &Error) {
        let mut err = err.clone();
        // A type in the message is printed in the names of the module that
        // failed, which need the scope over them just as `type_names` does.
        if let Error::Type { failed_modules, .. } = &mut err {
            for module in failed_modules.values_mut() {
                self.scope.register_types(&mut module.names);
            }
        }
        self.show_diagnostics(err.to_diagnostics(), Show::PreferOnInput);
    }

    /// The names a type is printed in: those of the module it was compiled in,
    /// plus the scope over them. Registering takes the plain name from whatever
    /// had it, so it goes to the newest definition, as it does for the user.
    fn type_names(&self, module: &Module) -> Names {
        let mut names = module.ast.names.clone();
        self.scope.register_types(&mut names);
        names
    }

    /// A warning the scaffolding causes rather than the input.
    fn is_noise(&self, warning: &Warning, purpose: &Purpose) -> bool {
        *purpose == Purpose::DeclareScope && is_repl_noise(warning)
    }

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

        // One that stayed put is about what the repl wrote: dropped outright
        // for a warning, and for an error only when another one lands on the
        // input, as an error with no place still says something.
        if show == Show::OnInputOnly || diags.iter().any(|(_, moved)| *moved) {
            diags.retain(|(_, moved)| *moved);
        }
        if diags.is_empty() {
            return;
        }

        let buffer_writer = crate::error::stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        for (diag, _) in &mut diags {
            // One that stayed put is about a module the user loaded, which
            // they know by the path they gave.
            relocate_to_user_paths(diag);
            diag.write(&mut buffer);
            writeln!(buffer).expect("write newline");
        }
        crate::error::flush_buffer(&buffer_writer, &buffer);
    }

    /// Move `diag` onto what the input it points into wrote, producing whether
    /// it pointed into one. Anything else stays where it is.
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
            // Every copy in a generated module is of the one input `loc.src`
            // now holds.
            debug_assert!(Rc::ptr_eq(extra_located.input, located.input));
            extra.label.span = extra_located.span;
            true
        });
        true
    }

    fn compile_and_run(&mut self, body: &Source) -> Result<Module, Error> {
        let module = self.compile_main(body, Purpose::Run)?;
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
    /// function per name, over the tuple the run memoized, and nothing else of
    /// the session — so no name of it can clash with one the input took.
    fn queue_vals_module(&mut self, from: &str, names: &[String]) -> String {
        // The plain name when the input has no definitions to hold it, so a
        // value is reached the way a type and a function are: `repl1.x()`.
        let plain = format!("repl{}", self.input_number);
        let module = if self.existing_modules.contains_key(plain.as_str()) {
            format!("repl{}_{}_vals", self.input_number, self.item_number)
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
        let module = self.compile_expr(input, expr, Purpose::Run)?;
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
    /// level, so the session keeps a function that runs the input once and
    /// remembers the tuple it produced. The value takes a name of the repl's
    /// before the pattern reads it, so the annotation and the pattern each go
    /// in once, checked where the user wrote them.
    fn run_assignment(
        &mut self,
        input: &Rc<str>,
        pattern: SrcSpan,
        annotation: Option<SrcSpan>,
        value: SrcSpan,
        message: Option<SrcSpan>,
        names: &[String],
    ) -> Result<(), InputError> {
        let (memo, print) = (self.repl_memo.clone(), self.repl_print.clone());
        let (vals, val) = (self.repl_vals.clone(), self.repl_value.clone());

        // Only the value and the message of a `let assert` read what the
        // session bound; a pattern names types and binds, and reads nothing.
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
        bind.write(&self.scope.injections(&reads, &[], &[]));
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

        // The value is remembered under the module's own name — never reused,
        // so no other input can fill it or read it.
        let module = self.module_name();
        let mut code = Source::new();
        code.write(&format!(
            "pub fn {vals}() {{\n{memo}(\"{module}\", fn() {{\n"
        ));
        code.append(&bind);
        code.write(&format!(
            "\n}})\n}}\n\npub fn {main}() {{\n{print}({vals}().0)\n}}\n",
            main = self.repl_main
        ));

        let vals_module = self.queue_vals_module(&module, names);
        let mut src = self.build_source(Some(code.as_str()), &Defined::default());
        src.append(&code);

        let module = self.compile(&module, src, Purpose::Run)?;
        self.run_repl_main(&module);

        if !self.engine.has_var(&module.name) {
            // The value raised before it was remembered, so nothing binds.
            return Ok(());
        }

        for name in names {
            self.scope.values.insert(
                name.clone(),
                NameEntry::new(&vals_module, name, Origin::Binding),
            );
        }
        self.scope.own_modules.push(vals_module);

        Ok(())
    }

    /// Compiles one expression, printed when it runs.
    fn compile_expr(
        &mut self,
        input: &Rc<str>,
        expr: SrcSpan,
        purpose: Purpose,
    ) -> Result<Module, Error> {
        let mut body = Source::new();
        body.write(&format!("{}({{\n", self.repl_print));
        body.copy(input, expr);
        body.write("\n})");
        self.compile_main(&body, purpose)
    }

    /// The one statement `:type` and `:time` take.
    fn command_statement(cmd: &str, input: &str) -> Result<UntypedStatement, InputError> {
        let refuse =
            |reason: &str| InputError::Repl(format!("{} command {reason}", cmd.trim_end()));
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
        self.item_number += 1;
        Self::command_statement(TYPE, expr)?;
        let input: Rc<str> = expr.into();
        let module =
            self.compile_expr(&input, SrcSpan::new(0, expr.len() as u32), Purpose::Type)?;
        let main = get_function(&module, &self.repl_main).expect("repl main function");
        println!(
            "{}",
            type_to_string(&self.type_names(&module), &main.return_type)
        );
        Ok(())
    }

    /// Takes a statement, not an expression, so a `let` under it binds.
    fn run_time_cmd(&mut self, expr: &str) -> Result<(), InputError> {
        self.item_number += 1;
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
        self.scope.register_import(import, input, span);
        let result = self.run_check();
        self.scope.own_import = None;
        result
    }

    /// The reason a const of the input cannot be accepted, when there is one.
    /// The parser accepts only a constant expression; the one thing it cannot
    /// know is that a name it reads is a `let`, out of reach from module level.
    fn const_refusal(&self, items: &[ReplItem]) -> Option<String> {
        // The qualified names go to `qualified` and stay there: only a plain
        // name can be a `let`.
        let (mut read, mut qualified) = (vec![], vec![]);
        for item in items {
            if let ReplItem::ReplDefinition(targeted, _) = item
                && let Definition::ModuleConstant(c) = &targeted.definition
            {
                constant_find_names(&c.value, &mut read, &mut qualified);
            }
        }
        let var = read.iter().find(|name| {
            self.scope
                .values
                .get(*name)
                .is_some_and(NameEntry::is_binding)
        })?;
        Some(format!(
            "`{var}` is a variable, not a constant. A constant can only use \
             literals, other constants and functions."
        ))
    }

    /// Compiles the definitions of the input into a module of its own, kept for
    /// the rest of the session, and binds each name to it. A later input
    /// imports the name instead, so a redefinition leaves the old one working.
    fn run_defs(&mut self, input: &Rc<str>, defs: &[Def]) -> Result<(), Error> {
        let defined = defined_by(defs);

        let mut code = Source::new();
        for def in defs {
            let text = &input[def.keyword as usize..def.span.end as usize];
            code.copy(input, SrcSpan::new(def.span.start, def.keyword));
            // Auto-pub, as the module it lands in is not the one that reads it.
            if def.private {
                code.write("pub ");
            }
            match &def.body {
                // The bindings go in ahead of the user's text, splitting the
                // definition in two copies of the input.
                Some(body) => {
                    code.copy(input, SrcSpan::new(def.keyword, body.start));
                    code.write(&self.scope.injections(text, &defined.values, &body.params));
                    code.copy(input, SrcSpan::new(body.start, def.span.end));
                }
                None => code.copy(input, SrcSpan::new(def.keyword, def.span.end)),
            }
            code.write("\n");
        }
        let mut src = self.build_source(Some(code.as_str()), &defined);
        src.append(&code);

        let module = self.defs_module_name();
        self.compile(&module, src, Purpose::Run)?;

        for def in defs {
            if let Some(name) = &def.type_name {
                self.scope.types.insert(
                    name.clone(),
                    NameEntry::new(&module, name, Origin::Def { reads: vec![] }),
                );
            }
            for name in &def.value_names {
                self.scope.values.insert(
                    name.clone(),
                    NameEntry::new(
                        &module,
                        name,
                        Origin::Def {
                            reads: def.reads.clone(),
                        },
                    ),
                );
            }
        }
        self.scope.own_modules.push(module);
        Ok(())
    }
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

/// The imports of the input, in the order it writes them, with what each
/// takes of it.
fn imports(items: &[ReplItem]) -> Vec<(&gleam_core::ast::Import<()>, SrcSpan)> {
    items
        .iter()
        .filter_map(|item| match item {
            ReplItem::ReplDefinition(targeted, start) => match &targeted.definition {
                Definition::Import(import) => {
                    Some((import, get_definition_span(&targeted.definition, *start)))
                }
                _ => None,
            },
            ReplItem::ReplStatement(_) => None,
        })
        .collect()
}

/// The definitions of the input, in the order it writes them.
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
                // An external function has no body to read the session from.
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
            for argument in arguments.iter().flatten() {
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
        Constant::Todo { message, .. } => {
            if let Some(message) = message {
                constant_find_names(message, names, modules);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_told_apart_from_gleam() {
        assert!(matches!(Command::parse(" :quit "), Command::Quit));
        assert!(matches!(Command::parse(":debug"), Command::Debug));
        assert!(matches!(Command::parse(":type 1"), Command::Type("1")));
        assert!(matches!(Command::parse(":time f()"), Command::Time("f()")));
        assert!(matches!(
            Command::parse(" 1 + 1 "),
            Command::Gleam(" 1 + 1 ")
        ));
    }

    #[test]
    fn a_near_miss_of_a_command_is_no_gleam() {
        for input in [":typ x", ":type", ":quit now", ":debug off"] {
            assert!(
                matches!(Command::parse(input), Command::Unknown(_)),
                "{input:?}"
            );
            assert!(!is_incomplete(input), "{input:?}");
        }
    }

    #[test]
    fn a_finished_input_is_run() {
        for input in ["1 + 1", "", "let x = 1", "pub fn f() {\n  1\n}", ":quit"] {
            assert_eq!(ready_state(input), -1, "{input:?}");
        }
    }

    #[test]
    fn an_unfinished_input_says_how_far_in_the_next_line_starts() {
        // Nothing is open: the input is waiting on a value, not on a bracket.
        assert_eq!(ready_state("let x ="), 0);
        assert_eq!(ready_state("1 +"), 0);
        // Every kind of bracket is worth a level.
        assert_eq!(ready_state("case x {"), 2);
        assert_eq!(ready_state("io.println("), 2);
        assert_eq!(ready_state("let x = [1,"), 2);
        assert_eq!(ready_state("let x = #(1,"), 2);
        assert_eq!(ready_state("pub fn f() {\n  case x {"), 4);
        // And one that is closed is worth none.
        assert_eq!(ready_state("pub fn f() {\n  f(1)\n  ["), 4);
        // A command is asked about what it carries.
        assert_eq!(ready_state(":type case x {"), 2);
    }

    #[test]
    fn a_blank_last_line_ends_an_input_with_nothing_open() {
        // The way out of an input that has no bracket left to type.
        assert_eq!(ready_state("let x =\n"), -1);
        assert_eq!(ready_state("let x = \"abc\n"), -1);
        assert_eq!(ready_state(":type 1 +\n"), -1);
        // A line with something on it is not one of these.
        assert_eq!(ready_state("1 +\n  2 +"), 0);
    }

    #[test]
    fn a_blank_line_inside_brackets_is_part_of_the_input() {
        // A function is written with blank lines between its statements, and
        // reading one as the end of the input cuts it in half.
        assert_eq!(ready_state("pub fn f() {\n  let x = 1\n"), 2);
        assert_eq!(ready_state("case x {\n  "), 2);
        assert_eq!(ready_state("let x = [\n  1,\n\n"), 2);
    }

    #[test]
    fn a_bad_command_is_answered_in_words() {
        assert_eq!(
            unknown_command(":quit now"),
            "The :quit command takes nothing after it."
        );
        assert_eq!(
            unknown_command(":type"),
            "The :type command takes an expression: `:type 1 + 1`."
        );
        assert!(unknown_command(":typ x").starts_with("Unknown command :typ."));
    }

    #[test]
    fn a_duration_is_said_in_its_own_unit() {
        use std::time::Duration;
        assert_eq!(format_duration(Duration::from_secs(2)), "2.00 s");
        assert_eq!(format_duration(Duration::from_millis(5)), "5 ms");
        assert_eq!(format_duration(Duration::from_micros(7)), "7 \u{b5}s");
        assert_eq!(format_duration(Duration::from_nanos(9)), "9 ns");
    }
}

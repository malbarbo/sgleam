use std::{fmt::Write, rc::Rc, time::Duration};

use ecow::EcoString;
use gleam_core::{
    Error,
    ast::{
        AssignName, BitArrayOption, BitArraySize, Constant, Definition, Pattern, SrcSpan,
        Statement, UntypedConstant, UntypedPattern, UntypedStatement,
    },
    build::Module,
    diagnostic::Diagnostic,
    io::{FileSystemReader, FileSystemWriter},
    type_::{ModuleInterface, printer::Names},
    warning::VectorWarningEmitterIO,
};
use indoc::formatdoc;

use crate::{
    engine::{Engine, MainFunction, ReplFile},
    error::SgleamError,
    gleam::{
        Project, get_definition_span, get_function, is_private, is_repl_noise, type_to_string,
    },
    parser::{self, ReplItem},
    scope::{Defined, NameEntry, Origin, Scope},
    source::Source,
    swriteln,
};

/// Why the repl compiled a module. Only a module compiled to run reaches the
/// runtime. A module that answers `:type` never runs, and nothing even names
/// the module that only declares the scope to check an import. That last one
/// uses nothing of the scope either, so an unused-import warning there says
/// nothing and the repl drops it.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Purpose {
    Run,
    Type,
    DeclareScope,
}

/// Which diagnostics reach the screen, after the repl has moved each one back
/// onto the input.
#[derive(PartialEq, Eq)]
enum Show {
    /// Only the ones that landed on the input. One that landed elsewhere is
    /// about text the repl wrote, and the user can do nothing about it.
    OnInputOnly,
    /// The rest as well, but only when none landed on the input. An error
    /// with no place still says something.
    PreferOnInput,
}

/// Why an input did not run.
enum InputError {
    Compile(Error),
    /// The input broke a rule of the repl, in words for a student.
    Repl(String),
}

impl From<Error> for InputError {
    fn from(error: Error) -> InputError {
        InputError::Compile(error)
    }
}

/// The input did not run, and why is already on the screen.
#[derive(Debug)]
pub struct Failed;

/// A definition of the input being run that goes to a module of its own,
/// instead of into every module the repl generates later.
struct Def {
    type_name: Option<String>,
    /// A function, a const, or the constructors of a type.
    value_names: Vec<String>,
    /// What a const reads from the scope, the module of a qualified name
    /// included.
    reads: Vec<String>,
    /// What it takes of the input, the attributes above it included.
    span: SrcSpan,
    /// Where `pub` goes. It comes after the attributes, and nothing may come
    /// between the two.
    keyword: u32,
    /// Whether the input left it private, which is what asks for the `pub`.
    /// The definition says so. The text before the keyword is either `pub`,
    /// spaced however the lexer allows, or nothing at all.
    private: bool,
    /// The body of a function, which is where the repl writes its bindings.
    body: Option<Body>,
}

/// Where the repl writes the bindings that a function body reads, and the
/// names already in the body.
struct Body {
    start: u32,
    params: Vec<String>,
}

/// The names the definitions bring in. Their own module must not import
/// them.
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
    /// The module that reads back the values of the item being run, compiled
    /// in the same pass as the module that computes them.
    pending_vals: Option<(String, Source)>,
    project: Project,
    existing_modules: im::HashMap<EcoString, ModuleInterface>,
    engine: E,
    // The input and the item being run. Their numbers name the module that
    // holds the compiled code.
    input_number: usize,
    item_number: usize,
    debug: bool,
    had_runtime_error: bool,
    /// What `:time` reports.
    elapsed: Duration,
    /// The modules the repl wrote for the item being run, by path, each one
    /// keeping which of its bytes are a copy of the input.
    generated: Vec<(camino::Utf8PathBuf, Source)>,
    /// Every module the repl wrote to run and has not yet handed to the
    /// runtime. A module that ran nothing can still raise later, from one of
    /// its functions.
    pending_files: Vec<ReplFile>,
    // A random suffix on each name, so no name of the user's can collide with
    // them.
    repl_main: String,
    repl_print: String,
    repl_memo: String,
    repl_vals: String,
    repl_value: String,
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
            engine: E::new(fs)?,
            input_number: 0,
            item_number: 0,
            debug: false,
            had_runtime_error: false,
            elapsed: Duration::ZERO,
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
        // One compilation here, so completion has the module interfaces to
        // read.
        repl.skip_taken_names();
        if let Err(error) = repl.run_check() {
            repl.show_gleam_error(&error);
        }
        Ok(repl)
    }

    /// The runtime that runs what the repl writes.
    pub fn engine(&self) -> &E {
        &self.engine
    }

    /// The completion candidates, in no order: every name in scope, and the
    /// public members of the imported modules, qualified. The shell adds its
    /// own and sorts what comes out.
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
        result
    }

    pub fn run(&mut self, src: &str) -> Result<(), Failed> {
        self.input(|repl| repl.run_source(src))
    }

    /// One statement, and how long the engine took on it.
    pub fn run_timed(&mut self, src: &str) -> Result<Duration, Failed> {
        self.input(|repl| {
            repl.guarded(|repl| {
                repl.item_number += 1;
                let statement = Self::one_statement(src)?;
                repl.run_statement(&src.into(), statement)?;
                Ok(repl.elapsed)
            })
        })
    }

    /// The type of one expression, without running it.
    pub fn type_of(&mut self, expr: &str) -> Result<String, Failed> {
        self.input(|repl| {
            repl.guarded(|repl| {
                // A command is an item of the input, not one of its
                // definitions. Item 0 names the module that reads back a
                // `let` of an input with no definitions.
                repl.item_number += 1;
                Self::one_statement(expr)?;
                let input: Rc<str> = expr.into();
                let body = repl.expr_body(&input, SrcSpan::new(0, expr.len() as u32));
                let module = repl.compile_main(&body, Purpose::Type)?;
                let main = get_function(&module, &repl.repl_main).expect("repl main function");
                Ok(type_to_string(&repl.type_names(&module), &main.return_type))
            })
        })
    }

    pub fn toggle_debug(&mut self) -> bool {
        self.debug = !self.debug;
        self.debug
    }

    fn input<T>(&mut self, run: impl FnOnce(&mut Self) -> Result<T, Failed>) -> Result<T, Failed> {
        self.had_runtime_error = false;
        // A failed input still spends its number. The repl never reuses a
        // module name, as the engine still holds the module it loaded under
        // that name.
        self.input_number += 1;
        self.item_number = 0;
        self.skip_taken_names();

        let result = run(self)?;
        if self.had_runtime_error {
            Err(Failed)
        } else {
            Ok(result)
        }
    }

    /// Runs one unit of the input — its definitions, or one of its items —
    /// undoing it if it does not go in. Only a unit that never ran leaves
    /// nothing behind.
    fn guarded<T>(
        &mut self,
        run: impl FnOnce(&mut Self) -> Result<T, InputError>,
    ) -> Result<T, Failed> {
        // The snapshot copies the names of the session and the text of the
        // modules just written. Engine and project count references internally
        // (Rc), so the clone leaves those shared, which also means it does not
        // roll them back. A module the engine loaded stays loaded, which is
        // fine, as nothing of the session names it anymore.
        let snapshot = (*self).clone();
        let error = match run(self) {
            Ok(result) => return Ok(result),
            Err(error) => error,
        };
        // This prints before the state goes back, as placing a diagnostic on
        // the input needs the modules the repl generated for it.
        let failed = self.report(error);
        *self = snapshot;
        Err(failed)
    }

    /// Says why an input did not run, and hands back the failure that says it
    /// already has.
    fn report(&self, error: InputError) -> Failed {
        match &error {
            InputError::Compile(error) => self.show_gleam_error(error),
            InputError::Repl(message) => println!("{message}"),
        }
        Failed
    }

    /// The definitions and the statements of an input, in the order it writes
    /// them. It stops at the first failure, as the user wrote what comes below
    /// expecting what comes above to have worked.
    fn run_source(&mut self, src: &str) -> Result<(), Failed> {
        // Reading the input changes nothing of the session, so there is
        // nothing to undo when it does not read.
        let input: Rc<str> = src.into();
        let items = parse(&input).map_err(|error| self.report(error.into()))?;
        if let Some(reason) = self.const_refusal(&items) {
            return Err(self.report(InputError::Repl(reason)));
        }

        // The imports go in ahead of everything else, as the compiler checks
        // a definition against the scope, and what its own input imported is
        // part of that scope.
        for (import, span) in imports(&items) {
            self.item_number += 1;
            self.guarded(|repl| {
                repl.run_import(import, &input, span)
                    .map_err(InputError::from)
            })?;
        }

        // The definitions go in next, in a module of their own, so they can
        // reference each other. All or nothing, which costs nothing, as no
        // item has run yet.
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
            // Whatever the item did before it raised stays, and its output
            // is already on the screen. `input` turns the flag into the
            // failure.
            if self.had_runtime_error {
                break;
            }
        }

        Ok(())
    }

    // --- Source generation ---

    /// What the input has in scope, as source: the externals, and the
    /// imports the scope writes for the names `code` mentions.
    fn build_source(&self, code: Option<&str>, skip: &Defined) -> Source {
        let mut src = Source::new();
        src.write(&self.build_externals());
        self.scope.write_imports(&mut src, code, skip);
        src
    }

    /// The FFI that lets the generated modules reach the engine.
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

    /// The module of the input itself. The user reads this name back in the
    /// type of a value a later redefinition left behind, so it stays plain.
    fn input_module(&self) -> String {
        format!("repl{}", self.input_number)
    }

    fn module_name(&self) -> String {
        match self.item_number {
            0 => self.input_module(),
            item => format!("{}_{item}", self.input_module()),
        }
    }

    fn write_source(&mut self, module_name: &str, code: &str) -> String {
        let file = format!("{module_name}.gleam");
        if self.debug {
            match crate::format::format_source(code) {
                Ok(formatted) => println!("--- {file} ---\n{formatted}---"),
                Err(_) => println!("--- {file} ---\n{code}\n---"),
            }
        }
        self.project.write_source(&file, code);
        file
    }

    /// Skips a number that a module of the user's already carries.
    /// `repl1.gleam` is a plausible file name, and the repl would write its own
    /// module over it.
    fn skip_taken_names(&mut self) {
        while self.name_taken() {
            self.input_number += 1;
        }
    }

    fn name_taken(&self) -> bool {
        let name = self.input_module();
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
        // The module that reads back the values of this item goes in here,
        // and not in a pass of its own, so a `let` costs one compilation.
        if let Some((vals_module, vals)) = self.pending_vals.take() {
            files.push(self.emit(&vals_module, vals, purpose));
        }
        files.push(self.emit(module_name, src, purpose));

        // The repl collects the warnings instead of printing each one as it
        // comes, so it can move them onto the input like errors.
        let warnings = VectorWarningEmitterIO::new();
        let result = self.project.compile_with_modules(
            Rc::new(warnings.clone()),
            &mut self.existing_modules,
            &mut im::HashMap::new(),
        );

        // The files go as soon as the compiler has them. The next input needs
        // the interface and the JavaScript, not the source.
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
                .filter(|warning| !(purpose == Purpose::DeclareScope && is_repl_noise(warning)))
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

    /// Writes one generated module out and keeps what the repl needs of it:
    /// which of its bytes are a copy of the input, so a diagnostic can go back
    /// onto it, and — for a module that will run — the input line behind each
    /// of its lines, which is all the runtime has to name a place with. Only a
    /// module compiled to run, as nothing ever reaches a place in the others,
    /// and loading them would cost the next run a module apiece.
    fn emit(&mut self, module_name: &str, src: Source, purpose: Purpose) -> String {
        let file = self.write_source(module_name, src.as_str());
        if purpose == Purpose::Run {
            self.pending_files.push(ReplFile {
                path: Project::source_path(&file).into_string(),
                lines: src.input_lines(),
            });
        }
        self.generated.push((Project::source().join(&file), src));
        file
    }

    /// Compiles `code` as a module of its own, with what it reads of the
    /// session written in front of it.
    fn compile_code(
        &mut self,
        module_name: &str,
        code: &Source,
        skip: &Defined,
        purpose: Purpose,
    ) -> Result<Module, Error> {
        let mut src = self.build_source(Some(code.as_str()), skip);
        src.append(code);
        self.compile(module_name, src, purpose)
    }

    fn compile_main(&mut self, body: &Source, purpose: Purpose) -> Result<Module, Error> {
        let mut code = Source::new();
        code.write(&format!("pub fn {}() {{\n", self.repl_main));
        code.write(&self.scope.injections(body.as_str(), &[], &[]));
        code.append(body);
        code.write("\n}\n");
        let module = self.module_name();
        self.compile_code(&module, &code, &Defined::default(), purpose)
    }

    fn show_gleam_error(&self, err: &Error) {
        let mut err = err.clone();
        // The message prints a type in the names of the module that failed,
        // and those names need the scope over them, just as `type_names` adds
        // it.
        if let Error::Type { failed_modules, .. } = &mut err {
            for module in failed_modules.values_mut() {
                self.scope.register_types(&mut module.names);
            }
        }
        self.show_diagnostics(err.to_diagnostics(), Show::PreferOnInput);
    }

    /// The names for printing a type: the names of the module that compiled
    /// it, plus the scope over them. Registering takes the plain name away from
    /// whatever held it, so the plain name lands on the newest definition,
    /// which is where the user expects it.
    fn type_names(&self, module: &Module) -> Names {
        let mut names = module.ast.names.clone();
        self.scope.register_types(&mut names);
        names
    }

    fn show_diagnostics(&self, diags: Vec<Diagnostic>, show: Show) {
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

        // A diagnostic that stayed put is about text the repl wrote. A
        // warning there goes outright, and an error there goes only when
        // another error lands on the input, as an error with no place still
        // says something.
        if show == Show::OnInputOnly || diags.iter().any(|(_, moved)| *moved) {
            diags.retain(|(_, moved)| *moved);
        }
        if diags.is_empty() {
            return;
        }

        // One that stayed put is about a module the user loaded, and the user
        // knows that module by the path they gave for it, which is what
        // printing puts back.
        let mut diags: Vec<Diagnostic> = diags.into_iter().map(|(diag, _)| diag).collect();
        crate::error::show_diagnostics(&mut diags);
    }

    /// Moves `diag` onto the input behind the generated module it points into,
    /// and returns `true` if it pointed into one. Anything else stays where it
    /// is.
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
        // An extra label that points at another file stays as it is. One that
        // points at this file stays only while it still says something about
        // the input.
        loc.extra_labels.retain_mut(|extra| {
            if extra.src_info.is_some() {
                return true;
            }
            let Some(extra_located) = src.locate(extra.label.span) else {
                return false;
            };
            // A generated module copies from one input only, and `loc.src`
            // now holds it.
            debug_assert!(Rc::ptr_eq(extra_located.input, located.input));
            extra.label.span = extra_located.span;
            true
        });
        true
    }

    /// One expression, wrapped so that running it prints its value.
    fn expr_body(&self, input: &Rc<str>, expr: SrcSpan) -> Source {
        let mut body = Source::new();
        body.write(&format!("{}({{\n", self.repl_print));
        body.copy(input, expr);
        body.write("\n})");
        body
    }

    /// Compiles a `repl_main` around `body` and runs it.
    fn run_body(&mut self, body: &Source) -> Result<(), InputError> {
        let module = self.compile_main(body, Purpose::Run)?;
        self.run_repl_main(&module);
        Ok(())
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

    /// Compiles without a `repl_main`, over the whole scope, which is how the
    /// repl checks an import together with the names it brought.
    fn run_check(&mut self) -> Result<(), Error> {
        let (module, src) = (
            self.module_name(),
            self.build_source(None, &Defined::default()),
        );
        self.compile(&module, src, Purpose::DeclareScope)
            .map(|_| ())
    }

    /// Builds the module that reads back the values an item binds: one
    /// function per name, over the tuple the run memoized, and nothing else of
    /// the session, so no name in it can clash with a name the input took.
    fn queue_vals_module(&mut self, from: &str, names: &[String]) -> String {
        // The plain name when the input has no definitions to hold it, so the
        // user reaches a value the way they reach a type or a function:
        // `repl1.x()`.
        let plain = self.input_module();
        let module = if self.existing_modules.contains_key(plain.as_str()) {
            format!("{}_vals", self.module_name())
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
                    // What a `let assert` writes after the value:
                    // `as "message"`.
                    let message =
                        (value.end < location.end).then(|| SrcSpan::new(value.end, location.end));
                    self.run_assignment(input, pattern, annotation, value, message, &names)
                }
            }
            Statement::Assert(_) => self.run_assert(input, location),
        }
    }

    fn run_expr(&mut self, input: &Rc<str>, expr: SrcSpan) -> Result<(), InputError> {
        let body = self.expr_body(input, expr);
        self.run_body(&body)
    }

    fn run_assert(&mut self, input: &Rc<str>, code: SrcSpan) -> Result<(), InputError> {
        let mut body = Source::new();
        body.copy(input, code);
        self.run_body(&body)
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
        let (memo, print) = (&self.repl_memo, &self.repl_print);
        let (vals, val) = (&self.repl_vals, &self.repl_value);

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

        // The run remembers the value under the module's own name. Nothing
        // reuses that name, so no other input can fill it or read it.
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
        let module = self.compile_code(&module, &code, &Defined::default(), Purpose::Run)?;
        self.run_repl_main(&module);

        if !self.engine.has_var(&module.name) {
            // The value raised before the run could remember it, so nothing
            // binds.
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

    fn one_statement(src: &str) -> Result<UntypedStatement, InputError> {
        let mut items = parse(src)?;
        if items.len() != 1 {
            return Err(InputError::Repl("Expected exactly one expression.".into()));
        }
        match items.swap_remove(0) {
            ReplItem::ReplStatement(statement) => Ok(statement),
            ReplItem::ReplDefinition(..) => Err(InputError::Repl(
                "Expected an expression, not a definition.".into(),
            )),
        }
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

    /// Why the repl cannot take a const of the input, when it cannot. The
    /// parser accepts only a constant expression, and the one thing it cannot
    /// know is that a name it reads is a `let`, out of reach from module
    /// level.
    fn const_refusal(&self, items: &[ReplItem]) -> Option<String> {
        // The qualified names go to `qualified` and stay there, as only a
        // plain name can be a `let`.
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
            // Auto-pub, as the module that reads the definition is not the
            // module that holds it.
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
        // Not `module_name`, as the imports are items and they went first.
        let module = self.input_module();
        self.compile_code(&module, &code, &defined, Purpose::Run)?;

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

/// The items of an input. The parser reads the input on its own, so an error
/// already points at it.
fn parse(input: &str) -> Result<Vec<ReplItem>, Error> {
    parser::parse_repl(input).map_err(|error| Error::Parse {
        path: "<repl>".into(),
        src: input.into(),
        error: error.into(),
    })
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
            // defines it, and the next input no longer reads that module.
            Definition::CustomType(t) if t.opaque => (Some(t.name.to_string()), vec![]),
            Definition::CustomType(t) => (
                Some(t.name.to_string()),
                t.constructors.iter().map(|c| c.name.to_string()).collect(),
            ),
            Definition::TypeAlias(t) => (Some(t.alias.to_string()), vec![]),
            Definition::Function(f) => {
                let name = f.name.clone().expect("A function must have a name").1;
                // An external function has no body, so nothing there reads
                // the session.
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
        // The compiler expands this into a `Record`, so the generated code
        // names the constructor.
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

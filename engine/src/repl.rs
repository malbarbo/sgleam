use std::{collections::BTreeMap, fmt::Write, rc::Rc};

use ecow::EcoString;
use gleam_core::{
    Error,
    ast::{
        AssignName, BitArrayOption, BitArraySize, Constant, Definition, Pattern, Statement,
        TargetedDefinition, UntypedConstant, UntypedPattern, UntypedStatement,
    },
    build::Module,
    diagnostic::Diagnostic,
    error::DefinedModuleOrigin,
    io::{FileSystemReader, FileSystemWriter},
    parse,
    type_::ModuleInterface,
    warning::VectorWarningEmitterIO,
};
use indoc::formatdoc;
use vec1::Vec1;

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{Project, get_definition_src, is_repl_noise, type_to_string},
    parser::{self, ReplItem},
    run::get_function,
    swrite, swriteln,
};

pub const QUIT: &str = ":quit";
pub const TYPE: &str = ":type ";
pub const TIME: &str = ":time ";
const DEBUG: &str = ":debug";

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d or \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

/// Lets the generated module use `const x = f()`, which the fork only accepts
/// while this is on. Kept off everywhere else so the student's own code, read
/// by `parser::parse_repl` and by `Project::compile`, still rejects it.
struct ConstCall;

impl ConstCall {
    fn enable() -> Self {
        parse::set_const_call_enabled(true);
        ConstCall
    }
}

impl Drop for ConstCall {
    fn drop(&mut self) {
        parse::set_const_call_enabled(false);
    }
}

/// Where a name in scope comes from: the module that defines it and the name
/// it has there. Every name reaches the input being run this way — an import,
/// a definition of an earlier input, or a `let` read back from its slot by the
/// companion module of the input that bound it.
#[derive(Clone)]
struct NameEntry {
    module: String,
    original: String,
    /// A `let`, the one kind of value a const may not read.
    runtime: bool,
}

impl NameEntry {
    fn defined_in(module: &str, original: impl AsRef<str>) -> NameEntry {
        NameEntry {
            module: module.into(),
            original: original.as_ref().into(),
            runtime: false,
        }
    }
}

/// A definition of the input being run that goes to a module of its own,
/// instead of being re-emitted into every module generated later.
struct Def {
    /// The name it binds as a type, when it is one.
    type_name: Option<String>,
    /// The names it binds as values: a function, a const, or the constructors
    /// of a type.
    value_names: Vec<String>,
    src: String,
}

impl Def {
    fn names(&self) -> impl Iterator<Item = &String> {
        self.type_name.iter().chain(&self.value_names)
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
    // Source of the definitions of the input being run.
    def_srcs: Vec<String>,
    // Modules holding the definitions of an input, in definition order.
    def_modules: Vec<String>,
    // The module of the values last bound, waiting for the next compilation.
    pending_vals: Option<(String, String)>,
    project: Project,
    existing_modules: im::HashMap<EcoString, ModuleInterface>,
    defined_modules: im::HashMap<EcoString, DefinedModuleOrigin>,
    engine: E,
    iter: (usize, usize),
    var_index: usize,
    debug: bool,
    had_runtime_error: bool,
    // Copied verbatim into the generated module, so diagnostics can be moved
    // back to it.
    user_text: Option<String>,
    // Internal function names with random suffix to avoid collisions with user code.
    repl_main: String,
    repl_print: String,
    repl_save: String,
    repl_load: String,
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
                .unwrap()
                .subsec_nanos()
        );
        let mut repl = Repl {
            modules: BTreeMap::new(),
            types: BTreeMap::new(),
            values: BTreeMap::new(),
            def_srcs: Vec::new(),
            def_modules: Vec::new(),
            pending_vals: None,
            project,
            existing_modules: im::HashMap::new(),
            defined_modules: im::HashMap::new(),
            engine: E::new(fs),
            iter: (0, 0),
            var_index: 0,
            debug: false,
            had_runtime_error: false,
            user_text: None,
            repl_main: format!("repl_main_{suffix}"),
            repl_print: format!("repl_print_{suffix}"),
            repl_save: format!("repl_save_{suffix}"),
            repl_load: format!("repl_load_{suffix}"),
        };
        if let Some(module) = user_module {
            repl.seed_module(module);
        }
        // Initial compilation, so the module interfaces completion reads are
        // available before the first input.
        let _ = repl.run_check();
        Ok(repl)
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

    pub fn run(&mut self, mut input: &str) -> Result<ReplOutput, SgleamError> {
        self.had_runtime_error = false;
        self.user_text = None;
        // Kept past the end of the run below, to relocate an error, so it is
        // this run that has to drop what the previous one left.
        self.def_srcs.clear();
        self.iter = (self.iter.0 + 1, 0);
        let line_trim = input.trim();

        if line_trim == QUIT {
            return Ok(ReplOutput::Quit);
        }

        if line_trim == DEBUG {
            self.debug = !self.debug;
            println!("Debug mode {}.", if self.debug { "on" } else { "off" });
            return Ok(ReplOutput::StdOut);
        }

        let mut is_type = false;
        let mut is_time = false;
        if let Some(expr) = line_trim.strip_prefix(TYPE) {
            input = expr;
            is_type = true;
        } else if let Some(expr) = line_trim.strip_prefix(TIME) {
            input = expr;
            is_time = true;
        }

        // The input is parsed on its own, so the error already points at it.
        let items = parser::parse_repl(input).map_err(|error| Error::Parse {
            path: "<repl>".into(),
            src: input.into(),
            error: error.into(),
        })?;

        if (is_type || is_time) && items.len() != 1 {
            let cmd = if is_type { TYPE } else { TIME };
            println!("{cmd}command expects exactly one expression.");
            return Ok(ReplOutput::StdOut);
        }

        let defs = if is_type || is_time {
            vec![]
        } else {
            defs(&items, input)
        };
        // Checked before anything runs, as the definitions of the input are
        // compiled together and the input either goes in whole or not at all.
        if let Some(reason) = self.const_refusal(&items) {
            println!("{reason}");
            return Ok(ReplOutput::StdOut);
        }

        // Snapshot for rollback: if any item fails, all changes from this
        // input are reverted. The clone is cheap — engine and project use
        // reference counting internally (Rc), so only the HashMaps are copied.
        let snapshot = (*self).clone();

        // The definitions go in first, together in a module of their own, so
        // they can reference each other and the items below only have to
        // import them.
        if !defs.is_empty()
            && let Err(err) = self.run_defs(&defs)
        {
            return Ok(self.rollback(snapshot, &err));
        }

        for item in items {
            self.iter.1 += 1;
            let result = match item {
                ReplItem::ReplDefinition(_) if is_type || is_time => {
                    let cmd = if is_type { TYPE } else { TIME };
                    println!("{cmd}command cannot be used with definitions.");
                    continue;
                }
                ReplItem::ReplDefinition(t) => self.run_definition(t, input),
                ReplItem::ReplStatement(_) if is_type => self.run_type_cmd(input),
                ReplItem::ReplStatement(_) if is_time => self.run_time_cmd(input),
                ReplItem::ReplStatement(s) => self.run_statement(s, input),
            };

            if let Err(err) = result {
                return Ok(self.rollback(snapshot, &err));
            }
        }

        if self.had_runtime_error {
            Ok(ReplOutput::Error)
        } else {
            Ok(ReplOutput::StdOut)
        }
    }

    // --- Source generation ---

    /// The modules in scope and the names taken from them, `skip` aside, which
    /// are the names the generated module defines itself.
    fn build_imports(&self, skip: &[String]) -> String {
        let mut src = String::new();
        for (name, path) in &self.modules {
            // A redundant alias would keep the line from being a
            // verbatim copy of the input, blocking relocation.
            if short_name(path) == name {
                swriteln!(src, "import {path}");
            } else {
                swriteln!(src, "import {path} as {name}");
            }
        }
        for (kind, entries) in [("", &self.values), ("type ", &self.types)] {
            for (
                name,
                NameEntry {
                    module, original, ..
                },
            ) in entries
            {
                if skip.contains(name) {
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

    /// The FFI the generated modules reach the engine through. Declared by
    /// every one of them: a constant of a companion module is inlined at its
    /// use, in a guard, and the inlined text names the loader.
    fn build_externals(&self) -> String {
        let (save, load, print) = (&self.repl_save, &self.repl_load, &self.repl_print);
        formatdoc! {r#"
            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_save")
            pub fn {save}(value: a) -> a

            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_load")
            pub fn {load}(index: Int) -> a

            @external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print")
            pub fn {print}(value: a) -> a
        "#}
    }

    /// Everything the input has in scope, as source: the externals and the
    /// imports. Every name, a saved value included, comes in by import — no
    /// annotation is written here, so nothing a later input redefines can
    /// change what this module reads.
    fn build_source(&self) -> String {
        let mut src = self.build_externals();
        src.push_str(&self.build_imports(&[]));
        src
    }

    // --- Compilation helpers ---

    /// The definitions of an input take the plain name, as the user reads it
    /// back in the type of a value the input that redefined the name left
    /// behind.
    fn module_name(&self) -> String {
        match self.iter {
            (input, 0) => format!("repl{input}"),
            (input, item) => format!("repl{input}_{item}"),
        }
    }

    /// Writes a module of this session, producing its file name.
    fn write_source(&mut self, module_name: &str, code: &str) -> String {
        let file = format!("{module_name}.gleam");
        if self.debug {
            let _const_call = ConstCall::enable();
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

    fn compile(&mut self, module_name: &str, code: &str) -> Result<Vec1<Module>, Error> {
        let mut files = vec![];
        // The module the values of the last item were saved into goes in here,
        // and not in a pass of its own: one pass compiles both, so a `let`
        // costs what any other input costs.
        if let Some((name, src)) = self.pending_vals.take() {
            files.push(self.write_source(&name, &src));
        }
        files.push(self.write_source(module_name, code));

        self.defined_modules.clear();
        // Collected instead of printed as they are emitted, so they can be
        // relocated like the errors.
        let warnings = VectorWarningEmitterIO::new();
        let result = {
            let _const_call = ConstCall::enable();
            self.project.compile_with_modules(
                Rc::new(warnings.clone()),
                &mut self.existing_modules,
                &mut self.defined_modules,
            )
        };

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
                .filter(|warning| !is_repl_noise(warning))
                .map(|warning| warning.to_diagnostic())
                .collect(),
            false,
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

        let mut modules1 = Vec1::new(modules.swap_remove(pos));
        modules1.extend(modules);

        Ok(modules1)
    }

    /// Compile source with a `repl_main` body appended.
    fn compile_main(&mut self, body: &str) -> Result<Module, Error> {
        let mut src = self.build_source();
        let repl_main = &self.repl_main;
        swrite!(src, "pub fn {repl_main}() {{\n{body}\n}}\n");
        let module = self.module_name();
        Ok(self.compile(&module, &src)?.split_off_first().0)
    }

    fn show_gleam_error(&self, err: &Error) {
        self.show_diagnostics(err.to_diagnostics(), true);
    }

    /// Reverts what the input did and shows `err`, keeping what it takes to
    /// place the error back on the input.
    fn rollback(&mut self, snapshot: Self, err: &Error) -> ReplOutput {
        let user_text = self.user_text.take();
        let def_srcs = std::mem::take(&mut self.def_srcs);
        *self = snapshot;
        self.user_text = user_text;
        self.def_srcs = def_srcs;
        self.show_gleam_error(err);
        ReplOutput::Error
    }

    /// Print diagnostics relocated to the user's input. `drop_unrelocated`
    /// discards the ones left pointing at the scaffolding, which for an error
    /// duplicate the relocated one.
    fn show_diagnostics(&self, diags: Vec<Diagnostic>, drop_unrelocated: bool) {
        use std::io::Write as _;
        if diags.is_empty() {
            return;
        }

        let mut diags: Vec<_> = diags
            .into_iter()
            .map(|mut diag| {
                let relocated = self.relocate(&mut diag);
                (diag, relocated)
            })
            .collect();

        if drop_unrelocated && diags.iter().any(|(_, relocated)| *relocated) {
            diags.retain(|(_, relocated)| *relocated);
        }

        let buffer_writer = crate::error::stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        for (diag, _) in &diags {
            diag.write(&mut buffer);
            writeln!(buffer).expect("write newline");
        }
        crate::error::flush_buffer(&buffer_writer, &buffer);
    }

    /// Move `diag` from the generated module back to the user's input,
    /// producing whether it could be done.
    fn relocate(&self, diag: &mut Diagnostic) -> bool {
        let Some(loc) = &mut diag.location else {
            return false;
        };
        let span = loc.label.span;
        // The definitions of this input are compiled together, in a module of
        // their own, so any of them can be the one holding the error.
        let Some((user_text, start)) =
            self.user_text
                .iter()
                .chain(&self.def_srcs)
                .find_map(|text| {
                    loc.src
                        .match_indices(text.as_str())
                        .map(|(i, _)| i as u32)
                        .find(|&i| i <= span.start && span.end <= i + text.len() as u32)
                        .map(|start| (text, start))
                })
        else {
            return false;
        };

        let end = start + user_text.len() as u32;
        loc.src = user_text.as_str().into();
        loc.path = "<repl>".into();
        loc.label.span.start -= start;
        loc.label.span.end -= start;
        loc.extra_labels.retain(|extra| {
            extra.src_info.is_some()
                || (start <= extra.label.span.start && extra.label.span.end <= end)
        });
        for extra in &mut loc.extra_labels {
            if extra.src_info.is_none() {
                extra.label.span.start -= start;
                extra.label.span.end -= start;
            }
        }
        true
    }

    /// Compile and execute a `repl_main` body.
    fn compile_and_run(&mut self, body: &str) -> Result<Module, Error> {
        let module = self.compile_main(body)?;
        self.run_repl_main(&module);
        Ok(module)
    }

    fn run_repl_main(&mut self, module: &Module) {
        if let Err(err) = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        ) {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        }
    }

    /// Compile without a `repl_main` (for checking definitions only).
    fn run_check(&mut self) -> Result<(), Error> {
        let (module, src) = (self.module_name(), self.build_source());
        self.compile(&module, &src).map(|_| ())
    }

    /// Builds the module the values an item bound are read back from, and binds
    /// each name to it. A `let` runs before its type is known, so the constant
    /// naming it can only be written after the run — and being written once, in
    /// the scope the type was printed in, it is never read again in a scope
    /// where its names mean something else. It is compiled by the next input,
    /// which is the first one that can read it.
    fn queue_vals_module(&mut self, bound: &[(String, String, usize)]) {
        let (input, item) = self.iter;
        let module = format!("repl{input}_{item}_vals");
        let load = self.repl_load.clone();
        let mut src = self.build_externals();

        let names: Vec<String> = bound.iter().map(|(name, ..)| name.clone()).collect();
        src.push_str(&self.build_imports(&names));
        // An annotation names the module of a type whose plain name a later
        // input took over, and that module is imported by no other line.
        for def_module in &self.def_modules {
            if self.modules.values().all(|path| path != def_module) {
                swriteln!(src, "import {def_module}");
            }
        }
        for (name, type_, index) in bound {
            swriteln!(src, "pub const {name}: {type_} = {load}({index})");
        }

        for (name, ..) in bound {
            self.values.insert(
                name.clone(),
                NameEntry {
                    module: module.clone(),
                    original: name.clone(),
                    runtime: true,
                },
            );
        }
        self.pending_vals = Some((module, src));
    }

    // --- Item handlers ---

    fn run_definition(&mut self, targeted: TargetedDefinition, src: &str) -> Result<(), Error> {
        match &targeted.definition {
            Definition::Import(import) => {
                self.user_text = Some(get_definition_src(&targeted.definition, src).into());
                self.run_import(import)
            }
            // Already compiled and bound by `run_defs`.
            _ => Ok(()),
        }
    }

    fn run_statement(&mut self, statement: UntypedStatement, src: &str) -> Result<(), Error> {
        let start = statement.location().start as usize;
        let end = statement.location().end as usize;

        match statement {
            Statement::Use(_) => {
                println!("use statements are not supported outside blocks.");
                Ok(())
            }
            Statement::Expression(_) => self.run_expr(&src[start..end]),
            Statement::Assignment(a) => {
                let mut names = vec![];
                assignment_find_names(&a.pattern, &mut names);
                if names.is_empty() {
                    let end = a.value.location().end as usize;
                    self.run_expr(&src[start..end])
                } else {
                    let pattern_end = a
                        .annotation
                        .as_ref()
                        .map_or(a.pattern.location().end, |t| t.location().end)
                        as usize;
                    let value_start = a.value.location().start as usize;
                    self.run_assignment(&src[start..pattern_end], &src[value_start..end], &names)
                }
            }
            Statement::Assert(_) => self.run_assert(&src[start..end]),
        }
    }

    fn run_expr(&mut self, expr: &str) -> Result<(), Error> {
        let print = &self.repl_print;
        let body = format!("{print}({{\n{expr}\n}})");
        self.user_text = Some(expr.into());
        self.compile_and_run(&body)?;
        Ok(())
    }

    fn run_assert(&mut self, code: &str) -> Result<(), Error> {
        self.user_text = Some(code.into());
        self.compile_and_run(code)?;
        Ok(())
    }

    fn run_assignment(
        &mut self,
        pattern: &str,
        value: &str,
        names: &[String],
    ) -> Result<(), Error> {
        let joined_names = names.join(", ");
        let (save, print) = (&self.repl_save, &self.repl_print);
        // Discarded through `let _` so saving a `Result` does not warn about an
        // unused value, a warning that would point at the generated module.
        let save_names = names
            .iter()
            .map(|name| format!("let _ = {save}({name})"))
            .collect::<Vec<_>>()
            .join("\n  ");
        // The inner binding is a verbatim copy of the input, so errors land on
        // it and not on the line carrying the print wrapper; the outer one
        // rebinds the names to save them and read their types back.
        let body = formatdoc! {"
          {pattern} = {print}({{
          {pattern} = {value}
          }})
          {save_names}
          #({joined_names})"
        };
        self.user_text = Some(format!("{pattern} = {value}"));
        let module = self.compile_and_run(&body)?;

        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, &self.repl_main).expect("repl main function");
            let types = main.return_type.tuple_types().unwrap();
            assert_eq!(types.len(), names.len());
            let bound: Vec<_> = names
                .iter()
                .zip(&types)
                .enumerate()
                .map(|(i, (name, type_))| {
                    (
                        name.clone(),
                        type_to_string(&module, type_),
                        self.var_index + i,
                    )
                })
                .collect();
            self.var_index += names.len();
            self.queue_vals_module(&bound);
        } else {
            // there was an error and the variable was not saved
        }

        Ok(())
    }

    fn run_type_cmd(&mut self, code: &str) -> Result<(), Error> {
        let print = &self.repl_print;
        let body = format!("{print}({{\n{code}\n}})");
        self.user_text = Some(code.into());
        let module = self.compile_main(&body)?;
        let main = &get_function(&module, &self.repl_main).expect("repl main function");
        println!("{}", type_to_string(&module, &main.return_type));
        Ok(())
    }
    fn run_time_cmd(&mut self, code: &str) -> Result<(), Error> {
        let print = &self.repl_print;
        let body = format!("{print}({{\n{code}\n}})");
        self.user_text = Some(code.into());
        let module = self.compile_main(&body)?;

        let start = std::time::Instant::now();
        let res = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        );
        let elapsed = start.elapsed();

        if let Err(err) = res {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        } else {
            let time_str = if elapsed.as_secs() > 0 {
                format!("{:.2} s", elapsed.as_secs_f64())
            } else if elapsed.as_millis() > 0 {
                format!("{} ms", elapsed.as_millis())
            } else if elapsed.as_micros() > 0 {
                format!("{} µs", elapsed.as_micros())
            } else {
                format!("{} ns", elapsed.as_nanos())
            };
            println!("Time: {time_str}");
        }
        Ok(())
    }

    fn run_import(&mut self, import: &gleam_core::ast::Import<()>) -> Result<(), Error> {
        let module = import.module.to_string();

        // Handle module alias / short name.
        match &import.as_name {
            Some((gleam_core::ast::AssignName::Variable(name), _)) => {
                self.modules.insert(name.to_string(), module.clone());
            }
            // `as _` brings in the unqualified names only.
            Some((gleam_core::ast::AssignName::Discard(_), _)) => {}
            None => {
                self.modules
                    .insert(short_name(&module).to_string(), module.clone());
            }
        }

        // Handle unqualified values
        for uv in &import.unqualified_values {
            let effective = uv
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| uv.name.to_string());
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
            self.types
                .insert(effective, NameEntry::defined_in(&module, &ut.name));
        }

        self.run_check()
    }

    /// The reason a const of the input cannot be accepted, when there is one.
    /// What makes it a const is otherwise already checked: the parser accepts
    /// only a constant expression, and the one thing the parser cannot know is
    /// that a name it reads is a `let`, out of reach from module level as it
    /// would be in a source file.
    fn const_refusal(&self, items: &[ReplItem]) -> Option<String> {
        let mut read = vec![];
        for item in items {
            if let ReplItem::ReplDefinition(targeted) = item
                && let Definition::ModuleConstant(c) = &targeted.definition
            {
                constant_find_names(&c.value, &mut read);
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
    fn run_defs(&mut self, defs: &[Def]) -> Result<(), Error> {
        let defined: Vec<String> = defs.iter().flat_map(Def::names).cloned().collect();

        let mut src = self.build_externals();
        src.push_str(&self.build_imports(&defined));
        for def in defs {
            // Auto-pub, as the module it lands in is not the one that reads it.
            if def.src.starts_with("pub ") {
                swriteln!(src, "{}", def.src);
            } else {
                swriteln!(src, "pub {}", def.src);
            }
        }

        self.def_srcs = defs.iter().map(|def| def.src.clone()).collect();
        let module = self.module_name();
        self.compile(&module, &src)?;

        for def in defs {
            if let Some(name) = &def.type_name {
                self.types
                    .insert(name.clone(), NameEntry::defined_in(&module, name));
            }
            for name in &def.value_names {
                self.values
                    .insert(name.clone(), NameEntry::defined_in(&module, name));
            }
        }
        self.def_modules.push(module);
        Ok(())
    }
}

/// The types, the functions and the consts the input defines, in the order it
/// defines them.
fn defs(items: &[ReplItem], input: &str) -> Vec<Def> {
    let mut defs = vec![];
    for item in items {
        let ReplItem::ReplDefinition(targeted) = item else {
            continue;
        };
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
                (None, vec![name.to_string()])
            }
            Definition::ModuleConstant(c) => (None, vec![c.name.to_string()]),
            Definition::Import(_) => continue,
        };
        defs.push(Def {
            type_name,
            value_names,
            src: get_definition_src(&targeted.definition, input).to_string(),
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

/// Collects the module level names a constant reads. Qualified ones are left
/// out: another module's name can never be a `let` from this session.
fn constant_find_names(constant: &UntypedConstant, names: &mut Vec<String>) {
    match constant {
        Constant::Int { .. }
        | Constant::Float { .. }
        | Constant::String { .. }
        | Constant::Invalid { .. } => {}
        Constant::Var { module, name, .. } => {
            if module.is_none() {
                names.push(name.into());
            }
        }
        Constant::Tuple { elements, .. } | Constant::List { elements, .. } => {
            for element in elements {
                constant_find_names(element, names);
            }
        }
        Constant::Record { arguments, .. } => {
            for argument in arguments {
                constant_find_names(&argument.value, names);
            }
        }
        Constant::Call {
            module,
            name,
            arguments,
            ..
        } => {
            if module.is_none() {
                names.push(name.into());
            }
            for argument in arguments {
                constant_find_names(&argument.value, names);
            }
        }
        Constant::RecordUpdate {
            record, arguments, ..
        } => {
            constant_find_names(&record.base, names);
            for argument in arguments {
                constant_find_names(&argument.value, names);
            }
        }
        Constant::BitArray { segments, .. } => {
            for segment in segments {
                constant_find_names(&segment.value, names);
                for option in &segment.options {
                    if let BitArrayOption::Size { value, .. } = option {
                        constant_find_names(value, names);
                    }
                }
            }
        }
        Constant::StringConcatenation { left, right, .. } => {
            constant_find_names(left, names);
            constant_find_names(right, names);
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

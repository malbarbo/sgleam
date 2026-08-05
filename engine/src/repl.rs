use std::{collections::BTreeMap, fmt::Write, rc::Rc};

use ecow::EcoString;
use gleam_core::{
    Error,
    ast::{
        AssignName, BitArraySize, Definition, Pattern, Statement, TargetedDefinition,
        UntypedPattern, UntypedStatement,
    },
    build::Module,
    diagnostic::Diagnostic,
    error::DefinedModuleOrigin,
    io::{FileSystemReader, FileSystemWriter},
    type_::ModuleInterface,
    warning::VectorWarningEmitterIO,
};
use indoc::formatdoc;
use vec1::Vec1;

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{Project, get_args_names, get_definition_src, is_repl_noise, type_to_string},
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

#[derive(Clone)]
enum NameEntry {
    /// `import gleam/int.{to_string}` → key "to_string"
    Unqualified { module: String, original: String },
    /// `type Color { Red }`
    Type(String),
    /// `let x = 10`, `fn f() { 1 }` or `const x = 1` (runtime value)
    Variable { index: usize, type_: String },
}

#[derive(Clone)]
pub struct Repl<E: Engine> {
    // One map per Gleam namespace, so a name in one cannot evict a name in
    // another: `import gleam/list`, `type list` and `fn list()` all coexist.
    //
    // BTreeMap (not HashMap) so the generated source lists imports, types and
    // consts in a stable, cross-run order — compiler diagnostics that
    // reference line numbers in it stay reproducible.
    /// `import gleam/int as i` → "i" → "gleam/int"
    modules: BTreeMap<String, String>,
    types: BTreeMap<String, NameEntry>,
    values: BTreeMap<String, NameEntry>,
    fn_bodies: BTreeMap<String, String>,
    // Verbatim source of the consts a new const may reference. Only
    // `run_const` emits it, as module-level scaffolding.
    consts: BTreeMap<String, String>,
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
            fn_bodies: BTreeMap::new(),
            consts: BTreeMap::new(),
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
            self.types.insert(
                type_.to_string(),
                NameEntry::Unqualified {
                    module: path.clone(),
                    original: type_.to_string(),
                },
            );
        }

        for value in interface.public_value_names() {
            self.bind_value(
                value.to_string(),
                NameEntry::Unqualified {
                    module: path.clone(),
                    original: value.to_string(),
                },
            );
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

        let items = parser::parse_repl(input).map_err(|error| Error::Parse {
            path: format!("/src/{}.gleam", self.module_name()).into(),
            src: input.into(),
            error: error.into(),
        })?;

        if (is_type || is_time) && items.len() != 1 {
            let cmd = if is_type { TYPE } else { TIME };
            println!("{cmd}command expects exactly one expression.");
            return Ok(ReplOutput::StdOut);
        }

        // Snapshot for rollback: if any item fails, all changes from this
        // input are reverted. Taken before registering the functions so a
        // rollback also drops them. The clone is cheap — engine and project use
        // reference counting internally (Rc), so only the HashMaps are copied.
        let snapshot = (*self).clone();

        // Pre-register function names so mutually recursive functions
        // can reference each other during compilation.
        for item in &items {
            if let ReplItem::ReplDefinition(targeted) = item
                && let Definition::Function(f) = &targeted.definition
            {
                let name = f.name.clone().expect("function name").1;
                let body = get_definition_src(&targeted.definition, input).into();
                self.fn_bodies.insert(name.into(), body);
            }
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
                let user_text = self.user_text.take();
                *self = snapshot;
                self.user_text = user_text;
                self.show_gleam_error(&err);
                return Ok(ReplOutput::Error);
            }
        }

        self.fn_bodies.clear();
        if self.had_runtime_error {
            Ok(ReplOutput::Error)
        } else {
            Ok(ReplOutput::StdOut)
        }
    }

    // --- Source generation ---

    fn build_source(&self) -> String {
        let mut src = String::new();
        let (save, load, print) = (&self.repl_save, &self.repl_load, &self.repl_print);
        swriteln!(
            src,
            r#"
@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_save")
pub fn {save}(value: a) -> a

@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_load")
pub fn {load}(index: Int) -> a

@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print")
pub fn {print}(value: a) -> a"#
        );

        // Imports
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
            for (name, entry) in entries {
                if let NameEntry::Unqualified { module, original } = entry {
                    if name == original {
                        swriteln!(src, "import {module}.{{{kind}{original}}} as _");
                    } else {
                        swriteln!(src, "import {module}.{{{kind}{original} as {name}}} as _");
                    }
                }
            }
        }

        // Types (auto-pub for REPL visibility)
        for item in self.types.values() {
            if let NameEntry::Type(code) = item {
                if code.starts_with("pub ") {
                    swriteln!(src, "{code}");
                } else {
                    swriteln!(src, "pub {code}");
                }
            }
        }

        // Function bodies
        for body in self.fn_bodies.values() {
            swriteln!(src, "{body}");
        }

        src
    }

    /// Generates variable load bindings for use inside function bodies.
    fn var_bindings(&self, exclude: &[String]) -> String {
        let mut bindings = String::new();
        for (name, item) in &self.values {
            if let NameEntry::Variable { index, type_ } = item
                && !exclude.contains(name)
            {
                let load = &self.repl_load;
                swriteln!(
                    bindings,
                    "  let {name} = fn () -> {type_} {{ {load}({index}) }} ()"
                );
            }
        }
        bindings
    }

    // --- Compilation helpers ---

    fn module_name(&self) -> String {
        format!("repl{}_{}", self.iter.0, self.iter.1)
    }

    fn compile(&mut self, code: &str) -> Result<Vec1<Module>, Error> {
        let module_name = self.module_name();
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

        self.defined_modules.clear();
        // Collected instead of printed as they are emitted, so they can be
        // relocated like the errors.
        let warnings = VectorWarningEmitterIO::new();
        let result = self.project.compile_with_modules(
            Rc::new(warnings.clone()),
            &mut self.existing_modules,
            &mut self.defined_modules,
        );

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

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
    /// Variable bindings are automatically included before the body.
    fn compile_main(&mut self, body: &str) -> Result<Module, Error> {
        let mut src = self.build_source();
        let (repl_main, bindings) = (&self.repl_main, self.var_bindings(&[]));
        swrite!(src, "pub fn {repl_main}() {{\n{bindings}{body}\n}}\n");
        Ok(self.compile(&src)?.split_off_first().0)
    }

    fn show_gleam_error(&self, err: &Error) {
        self.show_diagnostics(err.to_diagnostics(), true);
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
        let Some(user_text) = &self.user_text else {
            return false;
        };
        let Some(loc) = &mut diag.location else {
            return false;
        };
        let span = loc.label.span;
        let len = user_text.len() as u32;
        let Some(start) = loc
            .src
            .match_indices(user_text.as_str())
            .map(|(i, _)| i as u32)
            .find(|&i| i <= span.start && span.end <= i + len)
        else {
            return false;
        };

        let end = start + len;
        loc.src = user_text.as_str().into();
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

        if let Err(err) = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        ) {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        }
        Ok(module)
    }

    /// Compile without a `repl_main` (for checking definitions only).
    fn run_check(&mut self) -> Result<(), Error> {
        self.compile(&self.build_source()).map(|_| ())
    }

    /// Compile the current state plus `defs`, run it and store the value of
    /// `name`, producing whether it was stored.
    fn define_value(&mut self, defs: &str, name: &str) -> Result<bool, Error> {
        let mut src = self.build_source();
        let (repl_main, save) = (&self.repl_main, &self.repl_save);
        swrite!(src, "{defs}pub fn {repl_main}() {{\n{save}({name})\n}}\n");
        let module = self.compile(&src)?.split_off_first().0;

        if let Err(err) = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        ) {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        }
        if !self.engine.has_var(self.var_index) {
            return Ok(false);
        }

        let main = get_function(&module, &self.repl_main).expect("repl main function");
        let type_ = type_to_string(&module, &main.return_type);
        self.bind_value(
            name.into(),
            NameEntry::Variable {
                index: self.var_index,
                type_,
            },
        );
        self.var_index += 1;
        Ok(true)
    }

    fn bind_value(&mut self, name: String, entry: NameEntry) {
        self.take_name(&name);
        self.values.insert(name, entry);
    }

    /// Drops the const scaffolding when `name` leaves it: an entry left behind
    /// could reference the old meaning of `name`.
    fn take_name(&mut self, name: &str) {
        if self.consts.contains_key(name) {
            self.consts.clear();
        }
    }

    // --- Item handlers ---

    fn run_definition(&mut self, targeted: TargetedDefinition, src: &str) -> Result<(), Error> {
        let mut src = get_definition_src(&targeted.definition, src).into();

        match &targeted.definition {
            Definition::Import(import) => {
                self.user_text = Some(src);
                self.run_import(import)
            }
            Definition::TypeAlias(t) => self.run_type(t.alias.to_string(), src),
            Definition::CustomType(t) => self.run_type(t.name.to_string(), src),
            Definition::ModuleConstant(c) => self.run_const(c.name.to_string(), src),
            Definition::Function(f) => {
                let bindings = self.var_bindings(&get_args_names(f));
                let body_start = (f.body.first().unwrap().location().start
                    - targeted.definition.location().start)
                    as usize;

                if bindings.is_empty() {
                    self.user_text = Some(src.clone());
                } else {
                    // Only what follows the spliced bindings is still a
                    // verbatim copy of the input.
                    self.user_text = Some(src[body_start..].to_string());
                    src.insert_str(body_start, &format!("\n  {bindings}"));
                }

                let name = f.name.clone().expect("A function must have a name").1;
                self.run_fn(name.into(), src)
            }
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
        let save_names = names
            .iter()
            .map(|name| format!("{save}({name})"))
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
            for (name, type_) in names.iter().zip(&types) {
                let index = self.var_index;
                let type_ = type_to_string(&module, type_);
                self.bind_value(name.into(), NameEntry::Variable { index, type_ });
                self.var_index += 1;
            }
        } else {
            // there was an error and the variable was not saved
        }

        Ok(())
    }

    fn run_fn(&mut self, name: String, body: String) -> Result<(), Error> {
        // Remove any existing name entry to avoid conflicts during compilation
        // (e.g., an unqualified import for the same name).
        self.values.remove(&name);
        self.fn_bodies.insert(name.clone(), body);
        self.define_value("", &name)?;
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
        if let Some((gleam_core::ast::AssignName::Variable(name), _)) = &import.as_name {
            self.modules.insert(name.to_string(), module.clone());
        } else {
            self.modules
                .insert(short_name(&module).to_string(), module.clone());
        }

        // Handle unqualified values
        for uv in &import.unqualified_values {
            let effective = uv
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| uv.name.to_string());
            self.bind_value(
                effective,
                NameEntry::Unqualified {
                    module: module.clone(),
                    original: uv.name.to_string(),
                },
            );
        }

        // Handle unqualified types
        for ut in &import.unqualified_types {
            let effective = ut
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| ut.name.to_string());
            self.types.insert(
                effective,
                NameEntry::Unqualified {
                    module: module.clone(),
                    original: ut.name.to_string(),
                },
            );
        }

        self.run_check()
    }

    /// A const is compiled at module level, next to the consts it may
    /// reference, and stored as its value like a `let` or a `fn`. Module level
    /// is what rejects a reference to a runtime value.
    fn run_const(&mut self, name: String, code: String) -> Result<(), Error> {
        // Remove stale function body to avoid module-level name conflict
        // (e.g., `fn f() { 1 } const f = 10` in the same input).
        self.fn_bodies.remove(&name);
        // Before the scaffolding is read: a redefinition must not be emitted twice.
        self.take_name(&name);

        let mut defs = String::new();
        for scaffold in self.consts.values() {
            swriteln!(defs, "{scaffold}");
        }
        swriteln!(defs, "{code}");

        self.user_text = Some(code.clone());
        if self.define_value(&defs, &name)? {
            self.consts.insert(name, code);
        }
        Ok(())
    }

    fn run_type(&mut self, name: String, code: String) -> Result<(), Error> {
        if self.values.values().any(
            |item| matches!(item, NameEntry::Variable { type_, .. } if type_mentions(&name, type_)),
        ) {
            println!("Cannot redefine type `{name}` while variables of that type exist.");
            return Ok(());
        }
        self.user_text = Some(code.clone());
        self.types.insert(name, NameEntry::Type(code));
        self.run_check()
    }
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

/// Check if a type string mentions a type name as a whole word.
/// E.g. `type_mentions("Option", "Option(Int)")` is true,
/// but `type_mentions("In", "Int")` is false.
fn type_mentions(name: &str, type_: &str) -> bool {
    let mut rest = type_;
    while let Some(pos) = rest.find(name) {
        let before_ok = pos == 0 || !rest.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let end = pos + name.len();
        let after_ok = end >= rest.len() || !rest.as_bytes()[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        rest = &rest[pos + name.len()..];
    }
    false
}

fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

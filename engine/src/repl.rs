use std::{collections::HashMap, fmt::Write};

use camino::Utf8PathBuf;
use ecow::EcoString;
use gleam_core::{
    ast::{
        BitArraySize, Definition, Pattern, Statement, TargetedDefinition, UntypedPattern,
        UntypedStatement,
    },
    build::Module,
    io::{FileSystemReader, FileSystemWriter},
    type_::ModuleInterface,
    Error,
};
use indoc::formatdoc;
use vec1::Vec1;

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{get_args_names, get_definition_src, type_to_string, Project},
    parser::{self, ReplItem},
    run::get_function,
    swrite, swriteln, GLEAM_MODULES_NAMES,
};

pub const QUIT: &str = ":quit";
pub const TYPE: &str = ":type ";
const DEBUG: &str = ":debug";

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d or \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

#[derive(Clone)]
enum NameEntry {
    /// `import gleam/int as i` → key "i"
    ModuleAlias { path: String, members: Vec<String> },
    /// `import gleam/int.{to_string}` → key "to_string"
    UnqualifiedValue { module: String, original: String },
    /// `import gleam/option.{type Option}` → key "Option"
    UnqualifiedType { module: String, original: String },
    /// `const x = 1`
    Const(String),
    /// `type Color { Red }`
    Type(String),
    /// `let x = 10` or `fn f() { 1 }` (runtime value)
    Variable { index: usize, type_: String },
}

#[derive(Clone)]
pub struct Repl<E: Engine> {
    user_import: Option<String>,
    names: HashMap<String, NameEntry>,
    fn_bodies: HashMap<String, String>,
    project: Project,
    existing_modules: im::HashMap<EcoString, ModuleInterface>,
    defined_modules: im::HashMap<EcoString, Utf8PathBuf>,
    engine: E,
    iter: (usize, usize),
    var_index: usize,
    debug: bool,
    had_runtime_error: bool,
    template_offset: u32,
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
        let names: HashMap<String, NameEntry> = GLEAM_MODULES_NAMES
            .iter()
            .map(|s| {
                let short = s.rsplit('/').next().unwrap_or(s);
                (
                    short.to_string(),
                    NameEntry::ModuleAlias {
                        path: s.to_string(),
                        members: vec![],
                    },
                )
            })
            .collect();
        let fs = project.fs.clone();
        let suffix = format!(
            "{:08x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        );
        let mut repl = Repl {
            user_import: user_module.map(import_public_types_and_values),
            names,
            fn_bodies: HashMap::new(),
            project,
            existing_modules: im::HashMap::new(),
            defined_modules: im::HashMap::new(),
            engine: E::new(fs),
            iter: (0, 0),
            var_index: 0,
            debug: false,
            had_runtime_error: false,
            template_offset: 0,
            repl_main: format!("repl_main_{suffix}"),
            repl_print: format!("repl_print_{suffix}"),
            repl_save: format!("repl_save_{suffix}"),
            repl_load: format!("repl_load_{suffix}"),
        };
        // Initial compilation to populate module_members cache.
        let _ = repl.run_check();
        Ok(repl)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(String::as_str)
    }

    /// Returns all completion candidates: unqualified names, qualified
    /// module.member names, and does NOT include keywords/commands (those
    /// are added by the CLI).
    pub fn completions(&self) -> Vec<String> {
        let mut result: Vec<String> = self.names.keys().cloned().collect();
        for (alias, entry) in &self.names {
            if let NameEntry::ModuleAlias { members, .. } = entry {
                for member in members {
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

        let type_ = if let Some(expr) = line_trim.strip_prefix(TYPE) {
            input = expr;
            true
        } else {
            false
        };

        let items = parser::parse_repl(input).map_err(|error| Error::Parse {
            path: format!("/src/{}.gleam", self.module_name()).into(),
            src: input.into(),
            error: error.into(),
        })?;

        if type_ && items.len() != 1 {
            println!("{TYPE}command expects exactly one expression.");
            return Ok(ReplOutput::StdOut);
        }

        // Pre-register function names so mutually recursive functions
        // can reference each other during compilation.
        for item in &items {
            if let ReplItem::ReplDefinition(targeted) = item {
                if let Definition::Function(f) = &targeted.definition {
                    let name = f.name.clone().expect("function name").1;
                    let body = get_definition_src(&targeted.definition, input).into();
                    self.fn_bodies.insert(name.into(), body);
                }
            }
        }

        // Snapshot for rollback: if any item fails, all changes from this
        // input are reverted. The clone is cheap — engine and project use
        // reference counting internally (Rc), so only the HashMaps are copied.
        let snapshot = (*self).clone();

        for item in items {
            self.iter.1 += 1;
            let result = match item {
                ReplItem::ReplDefinition(_) if type_ => {
                    println!("{TYPE}command cannot be used with definitions.");
                    continue;
                }
                ReplItem::ReplDefinition(t) => self.run_definition(t, input),
                ReplItem::ReplStatement(_) if type_ => self.run_type_cmd(input),
                ReplItem::ReplStatement(s) => self.run_statement(s, input),
            };

            if let Err(err) = result {
                let template_offset = self.template_offset;
                *self = snapshot;
                self.template_offset = template_offset;
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
        if let Some(user) = &self.user_import {
            swriteln!(src, "{user}");
        }
        for (name, entry) in &self.names {
            match entry {
                NameEntry::ModuleAlias { path: module, .. } => {
                    swriteln!(src, "import {module} as {name}");
                }
                NameEntry::UnqualifiedValue { module, original } => {
                    if name == original {
                        swriteln!(src, "import {module}.{{{original}}} as _");
                    } else {
                        swriteln!(src, "import {module}.{{{original} as {name}}} as _");
                    }
                }
                NameEntry::UnqualifiedType { module, original } => {
                    if name == original {
                        swriteln!(src, "import {module}.{{type {original}}} as _");
                    } else {
                        swriteln!(src, "import {module}.{{type {original} as {name}}} as _");
                    }
                }
                _ => {}
            }
        }

        // Consts
        for item in self.names.values() {
            if let NameEntry::Const(code) = item {
                swriteln!(src, "{code}");
            }
        }

        // Types (auto-pub for REPL visibility)
        for item in self.names.values() {
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
        for (name, item) in &self.names {
            if let NameEntry::Variable { index, type_ } = item {
                if !exclude.contains(name) {
                    let load = &self.repl_load;
                    swriteln!(
                        bindings,
                        "  let {name} = fn () -> {type_} {{ {load}({index}) }} ()"
                    );
                }
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

        let result = self.project.compile_with_modules(
            true,
            &mut self.existing_modules,
            &mut self.defined_modules,
        );

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

        let mut modules = result?;

        // Fill in empty members for ModuleAlias entries from compiled module interfaces.
        for entry in self.names.values_mut() {
            if let NameEntry::ModuleAlias { path, members } = entry {
                if members.is_empty() {
                    if let Some(iface) = self.existing_modules.get(path.as_str()) {
                        *members = iface
                            .public_value_names()
                            .into_iter()
                            .map(String::from)
                            .chain(iface.public_type_names().into_iter().map(String::from))
                            .collect();
                        members.sort();
                    }
                }
            }
        }

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
    /// `body_prefix` is the number of bytes in `body` before the user's code
    /// (e.g., the `repl_print({ ` wrapper), used to adjust error positions.
    fn compile_main(&mut self, body: &str, body_prefix: usize) -> Result<Module, Error> {
        self.compile_main_with_bindings(&self.var_bindings(&[]), body, body_prefix)
    }

    /// Like `compile_main` but with custom variable bindings (to exclude
    /// function argument names).
    fn compile_main_with_bindings(
        &mut self,
        bindings: &str,
        body: &str,
        body_prefix: usize,
    ) -> Result<Module, Error> {
        let mut src = self.build_source();
        let repl_main = &self.repl_main;
        let header = format!("pub fn {repl_main}() {{\n{bindings}");
        self.template_offset = (src.len() + header.len() + body_prefix) as u32;
        src.push_str(&header);
        src.push_str(body);
        src.push_str("\n}\n");
        Ok(self.compile(&src)?.split_off_first().0)
    }

    /// Display a compile error with adjusted line numbers.
    fn show_gleam_error(&self, err: &Error) {
        use std::io::Write as _;
        let offset = self.template_offset;
        let buffer_writer = crate::error::stderr_buffer_writer();
        let mut buffer = buffer_writer.buffer();
        for mut diag in err.to_diagnostics() {
            if let Some(ref mut loc) = diag.location {
                if loc.label.span.start >= offset {
                    loc.src = loc.src[offset as usize..].into();
                    loc.label.span.start -= offset;
                    loc.label.span.end -= offset;
                    for extra in &mut loc.extra_labels {
                        if extra.src_info.is_none() {
                            extra.label.span.start -= offset;
                            extra.label.span.end -= offset;
                        }
                    }
                }
            }
            diag.write(&mut buffer);
            writeln!(buffer).expect("write newline");
        }
        crate::error::flush_buffer(&buffer_writer, &buffer);
    }

    /// Compile and execute a `repl_main` body.
    fn compile_and_run(&mut self, body: &str, body_prefix: usize) -> Result<Module, Error> {
        let module = self.compile_main(body, body_prefix)?;

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

    // --- Item handlers ---

    fn run_definition(&mut self, targeted: TargetedDefinition, src: &str) -> Result<(), Error> {
        let mut src = get_definition_src(&targeted.definition, src).into();

        match &targeted.definition {
            Definition::Import(import) => self.run_import(import),
            Definition::TypeAlias(t) => self.run_type(t.alias.to_string(), src),
            Definition::CustomType(t) => self.run_type(t.name.to_string(), src),
            Definition::ModuleConstant(c) => self.run_const(c.name.to_string(), src),
            Definition::Function(f) => {
                let bindings = self.var_bindings(&get_args_names(f));

                src.insert_str(
                    (f.body.first().unwrap().location().start
                        - targeted.definition.location().start) as usize,
                    &format!("\n  {bindings}"),
                );

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
                    let pattern_end = a.pattern.location().end as usize;
                    let value_start = a.value.location().start as usize;
                    self.run_assignment(&src[start..pattern_end], &src[value_start..end], &names)
                }
            }
            Statement::Assert(_) => self.run_assert(&src[start..end]),
        }
    }

    fn run_expr(&mut self, expr: &str) -> Result<(), Error> {
        let print = &self.repl_print;
        let prefix = format!("{print}({{\n");
        let body = format!("{prefix}{expr}\n}})");
        self.compile_and_run(&body, prefix.len())?;
        Ok(())
    }

    fn run_assert(&mut self, code: &str) -> Result<(), Error> {
        self.compile_and_run(code, 0)?;
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
        let body = formatdoc! {"
          {pattern} = {print}({value})
          {save_names}
          #({joined_names})"
        };
        let module = self.compile_and_run(&body, 0)?;

        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, &self.repl_main).expect("repl main function");
            let types = main.return_type.tuple_types().unwrap();
            assert_eq!(types.len(), names.len());
            for (name, type_) in names.iter().zip(&types) {
                let index = self.var_index;
                let type_ = type_to_string(&module, type_);
                self.names
                    .insert(name.into(), NameEntry::Variable { index, type_ });
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
        self.names.remove(&name);
        self.fn_bodies.insert(name.clone(), body);
        let save = &self.repl_save;
        let body = format!("{save}({name})");
        let module = self.compile_main_with_bindings("", &body, 0)?;
        if let Err(err) = self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        ) {
            crate::error::show_error(&err);
            self.had_runtime_error = true;
        }
        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, &self.repl_main).expect("repl main function");
            let type_ = type_to_string(&module, &main.return_type);
            self.names.insert(
                name,
                NameEntry::Variable {
                    index: self.var_index,
                    type_,
                },
            );
            self.var_index += 1;
        }
        Ok(())
    }

    fn run_type_cmd(&mut self, code: &str) -> Result<(), Error> {
        let print = &self.repl_print;
        let body = formatdoc! {"
          {print}({{
            {code}
          }})"
        };
        let module = self.compile_main(&body, 0)?;
        let main = &get_function(&module, &self.repl_main).expect("repl main function");
        println!("{}", type_to_string(&module, &main.return_type));
        Ok(())
    }

    fn run_import(&mut self, import: &gleam_core::ast::Import<()>) -> Result<(), Error> {
        let module = import.module.to_string();

        // Handle module alias / short name.
        // Members are populated after the next compile() call.
        let alias_entry = NameEntry::ModuleAlias {
            path: module.clone(),
            members: vec![],
        };
        if let Some((gleam_core::ast::AssignName::Variable(name), _)) = &import.as_name {
            self.names.insert(name.to_string(), alias_entry);
        } else {
            let short = module.rsplit('/').next().unwrap_or(&module).to_string();
            self.names.insert(short, alias_entry);
        }

        // Handle unqualified values
        for uv in &import.unqualified_values {
            let effective = uv
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| uv.name.to_string());
            self.names.insert(
                effective,
                NameEntry::UnqualifiedValue {
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
            self.names.insert(
                effective,
                NameEntry::UnqualifiedType {
                    module: module.clone(),
                    original: ut.name.to_string(),
                },
            );
        }

        self.run_check()
    }

    fn run_const(&mut self, name: String, code: String) -> Result<(), Error> {
        // Remove stale function body to avoid module-level name conflict
        // (e.g., `fn f() { 1 } const f = 10` in the same input).
        self.fn_bodies.remove(&name);
        self.names.insert(name, NameEntry::Const(code));
        self.run_check()
    }

    fn run_type(&mut self, name: String, code: String) -> Result<(), Error> {
        if self.names.values().any(
            |item| matches!(item, NameEntry::Variable { type_, .. } if type_mentions(&name, type_)),
        ) {
            println!("Cannot redefine type `{name}` while variables of that type exist.");
            return Ok(());
        }
        self.names.insert(name, NameEntry::Type(code));
        self.run_check()
    }
}

fn assignment_find_names(pattern: &UntypedPattern, names: &mut Vec<String>) {
    match pattern {
        Pattern::Int { .. }
        | Pattern::Float { .. }
        | Pattern::String { .. }
        | Pattern::Discard { .. }
        | Pattern::Invalid { .. }
        | Pattern::StringPrefix { .. } => {}
        Pattern::Variable { name, .. } => names.push(name.into()),
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

fn import_public_types_and_values(module: &Module) -> String {
    let mut import = String::new();
    let name = &module.name;
    swrite!(&mut import, "import {name}.{{");
    for type_ in module.ast.type_info.public_type_names() {
        swrite!(&mut import, "type {type_}, ");
    }
    for value in module.ast.type_info.public_value_names() {
        swrite!(&mut import, "{value}, ");
    }
    import.push('}');
    import
}

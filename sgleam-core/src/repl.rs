use std::{collections::HashMap, fmt::Write};

use gleam_core::{
    ast::{
        BitArraySize, Definition, Pattern, Statement, TargetedDefinition, UntypedPattern,
        UntypedStatement,
    },
    build::Module,
    io::{FileSystemReader, FileSystemWriter},
    Error,
};
use indoc::formatdoc;
use vec1::Vec1;

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{compile, get_args_names, get_definition_src, type_to_string, Project},
    parser::{self, ReplItem},
    run::get_function,
    swrite, swriteln, GLEAM_MODULES_NAMES,
};

pub const QUIT: &str = ":quit";
pub const TYPE: &str = ":type ";
const DEBUG: &str = ":debug";

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d ou \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

#[derive(Clone)]
struct UnqualifiedItem {
    name: String,
    as_name: Option<String>,
}

#[derive(Clone, Default)]
struct ImportInfo {
    as_name: Option<String>,
    unqualified_values: Vec<UnqualifiedItem>,
    unqualified_types: Vec<UnqualifiedItem>,
}

#[derive(Clone)]
enum NameItem {
    Const(String),
    Type(String),
    Variable { index: usize, type_: String },
}

#[derive(Clone)]
pub struct Repl<E: Engine> {
    user_import: Option<String>,
    imports: HashMap<String, ImportInfo>,
    names: HashMap<String, NameItem>,
    fn_bodies: HashMap<String, String>,
    project: Project,
    engine: E,
    iter: (usize, usize),
    var_index: usize,
    debug: bool,
    template_offset: u32,
    // Internal function names with random suffix to avoid collisions with user code.
    repl_main: String,
    repl_print: String,
    repl_save: String,
    repl_load: String,
}

pub enum ReplOutput {
    Quit,
    StdOut,
}

impl<E: Engine> Repl<E> {
    pub fn new(project: Project, user_module: Option<&Module>) -> Result<Repl<E>, SgleamError> {
        let imports: HashMap<String, ImportInfo> = GLEAM_MODULES_NAMES
            .iter()
            .map(|s| (s.to_string(), ImportInfo::default()))
            .collect();
        let fs = project.fs.clone();
        let suffix = format!(
            "{:08x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        );
        Ok(Repl {
            user_import: user_module.map(import_public_types_and_values),
            imports,
            names: HashMap::new(),
            fn_bodies: HashMap::new(),
            project,
            engine: E::new(fs),
            iter: (0, 0),
            var_index: 0,
            debug: false,
            template_offset: 0,
            repl_main: format!("repl_main_{suffix}"),
            repl_print: format!("repl_print_{suffix}"),
            repl_save: format!("repl_save_{suffix}"),
            repl_load: format!("repl_load_{suffix}"),
        })
    }

    pub fn run(&mut self, mut input: &str) -> Result<ReplOutput, SgleamError> {
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
                return Ok(ReplOutput::StdOut);
            }
        }

        self.fn_bodies.clear();
        Ok(ReplOutput::StdOut)
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
        for (module, info) in &self.imports {
            let mut parts = vec![];
            for item in &info.unqualified_types {
                if let Some(alias) = &item.as_name {
                    parts.push(format!("type {} as {alias}", item.name));
                } else {
                    parts.push(format!("type {}", item.name));
                }
            }
            for item in &info.unqualified_values {
                if let Some(alias) = &item.as_name {
                    parts.push(format!("{} as {alias}", item.name));
                } else {
                    parts.push(item.name.clone());
                }
            }
            let unqualified = if parts.is_empty() {
                String::new()
            } else {
                format!(".{{{}}}", parts.join(", "))
            };
            let as_clause = info
                .as_name
                .as_ref()
                .map(|n| format!(" as {n}"))
                .unwrap_or_default();
            swriteln!(src, "import {module}{unqualified}{as_clause}");
        }

        // Consts
        for item in self.names.values() {
            if let NameItem::Const(code) = item {
                swriteln!(src, "{code}");
            }
        }

        // Types (auto-pub for REPL visibility)
        for item in self.names.values() {
            if let NameItem::Type(code) = item {
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
            if let NameItem::Variable { index, type_ } = item {
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

        let result = compile(&mut self.project, true);

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

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
        #[cfg(feature = "capture")]
        crate::quickjs::write_stderr(&String::from_utf8_lossy(buffer.as_slice()));
        #[cfg(not(feature = "capture"))]
        buffer_writer.print(&buffer).expect("write error");
    }

    /// Compile and execute a `repl_main` body.
    fn compile_and_run(&mut self, body: &str, body_prefix: usize) -> Result<Module, Error> {
        let module = self.compile_main(body, body_prefix)?;

        self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        );
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
                    .insert(name.into(), NameItem::Variable { index, type_ });
                self.var_index += 1;
            }
        } else {
            // there was an error and the variable was not saved
        }

        Ok(())
    }

    fn run_fn(&mut self, name: String, body: String) -> Result<(), Error> {
        self.remove_imported_value(&name);
        self.fn_bodies.insert(name.clone(), body);
        let save = &self.repl_save;
        let body = format!("{save}({name})");
        let module = self.compile_main_with_bindings("", &body, 0)?;
        self.engine.run_main(
            &module.name,
            MainFunction::ReplMain(self.repl_main.clone()),
            false,
        );
        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, &self.repl_main).expect("repl main function");
            let type_ = type_to_string(&module, &main.return_type);
            self.names.insert(
                name,
                NameItem::Variable {
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

        // Remove unqualified names from other modules to avoid duplicates
        for uv in &import.unqualified_values {
            let effective = uv
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| uv.name.to_string());
            self.remove_imported_value(&effective);
        }
        for ut in &import.unqualified_types {
            let effective = ut
                .as_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| ut.name.to_string());
            self.remove_imported_type(&effective);
        }

        // Rename any existing import with the same short name to avoid conflicts
        if import.as_name.is_none() {
            let short = module.rsplit('/').next().unwrap_or(&module);
            for (m, info) in &mut self.imports {
                if *m != module
                    && info.as_name.is_none()
                    && m.rsplit('/').next().unwrap_or(m) == short
                {
                    info.as_name = Some("_".into());
                }
            }
        }

        let entry = self.imports.entry(module).or_default();

        if let Some((gleam_core::ast::AssignName::Variable(name), _)) = &import.as_name {
            entry.as_name = Some(name.to_string());
        } else {
            entry.as_name = None;
        }

        for uv in &import.unqualified_values {
            let name = uv.name.to_string();
            if !entry.unqualified_values.iter().any(|i| i.name == name) {
                entry.unqualified_values.push(UnqualifiedItem {
                    name,
                    as_name: uv.as_name.as_ref().map(|n| n.to_string()),
                });
            }
        }

        for ut in &import.unqualified_types {
            let name = ut.name.to_string();
            if !entry.unqualified_types.iter().any(|i| i.name == name) {
                entry.unqualified_types.push(UnqualifiedItem {
                    name,
                    as_name: ut.as_name.as_ref().map(|n| n.to_string()),
                });
            }
        }

        self.run_check()
    }

    fn run_const(&mut self, name: String, code: String) -> Result<(), Error> {
        // Remove stale function body to avoid module-level name conflict
        // (e.g., `fn f() { 1 } const f = 10` in the same input).
        self.fn_bodies.remove(&name);
        self.names.insert(name, NameItem::Const(code));
        self.run_check()
    }

    fn run_type(&mut self, name: String, code: String) -> Result<(), Error> {
        if self
            .names
            .values()
            .any(|item| matches!(item, NameItem::Variable { type_, .. } if type_.contains(&name)))
        {
            println!("Cannot redefine type `{name}` while variables of that type exist.");
            return Ok(());
        }
        self.remove_imported_type(&name);
        self.names.insert(name, NameItem::Type(code));
        self.run_check()
    }

    fn remove_imported_value(&mut self, name: &str) {
        for info in self.imports.values_mut() {
            info.unqualified_values
                .retain(|i| i.name != name && i.as_name.as_deref() != Some(name));
        }
    }

    fn remove_imported_type(&mut self, name: &str) {
        for info in self.imports.values_mut() {
            info.unqualified_types
                .retain(|i| i.name != name && i.as_name.as_deref() != Some(name));
        }
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

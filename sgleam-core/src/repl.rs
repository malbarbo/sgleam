use std::{collections::HashMap, fmt::Write};

use gleam_core::{
    ast::{
        BitArraySize, Definition, Pattern, Statement, TargetedDefinition, UntypedPattern,
        UntypedStatement,
    },
    build::Module,
    io::FileSystemWriter,
    Error,
};
use indoc::formatdoc;
use vec1::Vec1;

use crate::{
    engine::{Engine, MainFunction, REPL_MAIN},
    error::SgleamError,
    gleam::{compile, get_args_names, get_definition_src, type_to_string, Project},
    parser::{self, ReplItem},
    run::get_function,
    swrite, swriteln, GLEAM_MODULES_NAMES,
};

const REPL_SAVE_LOAD_FNS: &str = r#"
@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_save")
pub fn repl_save(value: a) -> a

@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_load")
pub fn repl_load(index: Int) -> a

@external(javascript, "./sgleam/sgleam_ffi.mjs", "repl_print")
pub fn repl_print(value: a) -> a
"#;

pub const QUIT: &str = ":quit";
pub const TYPE: &str = ":type ";

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d ou \"{QUIT}\" to exit.\n",
        crate::version()
    )
}

#[derive(Clone)]
pub struct Repl<E: Engine> {
    user_import: Option<String>,
    imports: Vec<String>,
    consts: Vec<String>,
    types: Vec<String>,
    fns: HashMap<String, Function>,
    vars: HashMap<String, Variable>,
    project: Project,
    engine: E,
    iter: (usize, usize),
    var_index: usize,
}

pub enum ReplOutput {
    Quit,
    StdOut,
}

#[derive(Clone)]
struct Variable {
    index: usize,
    type_: String,
}

#[derive(Clone)]
struct Function {
    index: usize,
    body: String,
}

impl<E: Engine> Repl<E> {
    pub fn new(project: Project, user_module: Option<&Module>) -> Result<Repl<E>, SgleamError> {
        let imports = GLEAM_MODULES_NAMES.iter().map(|s| s.to_string()).collect();
        let fs = project.fs.clone();
        Ok(Repl {
            user_import: user_module.map(import_public_types_and_values),
            imports,
            consts: vec![],
            types: vec![],
            fns: HashMap::new(),
            vars: HashMap::new(),
            project,
            engine: E::new(fs),
            iter: (0, 0),
            var_index: 0,
        })
    }

    pub fn run(&mut self, mut input: &str) -> Result<ReplOutput, SgleamError> {
        self.iter = (self.iter.0 + 1, 0);
        let line_trim = input.trim();

        if line_trim == QUIT {
            return Ok(ReplOutput::Quit);
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

        // FIXME: avoid this clone
        // We clone self so we can rollback if the execution fail
        let repl = (*self).clone();

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
                *self = repl;
                return Err(SgleamError::Gleam(err));
            }
        }

        Ok(ReplOutput::StdOut)
    }

    fn build_source(&self) -> String {
        let mut src = String::new();
        src.push_str(REPL_SAVE_LOAD_FNS);
        self.add_imports(&mut src);
        self.add_consts(&mut src);
        self.add_types(&mut src);
        self.add_fns(&mut src);
        src
    }

    fn module_name(&self) -> String {
        format!("repl{}_{}", self.iter.0, self.iter.1)
    }

    fn compile(&mut self, code: &str) -> Result<Vec1<Module>, Error> {
        let module_name = self.module_name();
        let file = format!("{module_name}.gleam");

        // TODO: add an option to show the generated code
        self.project.write_source(&file, code);

        let result = compile(&mut self.project, true);

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

        let mut modules = result?;

        let pos = modules
            .iter()
            .position(|module| module.name == module_name)
            .expect("The repl module");

        let mut modules1 = Vec1::new(modules.swap_remove(pos));
        modules1.extend(modules);

        Ok(modules1)
    }

    fn run_definition(&mut self, targeted: TargetedDefinition, src: &str) -> Result<(), Error> {
        let mut src = get_definition_src(&targeted.definition, src).into();

        match &targeted.definition {
            Definition::Import(_) => self.run_import(src),
            Definition::TypeAlias(_) | Definition::CustomType(_) => self.run_type(src),
            Definition::ModuleConstant(_) => self.run_const(src),
            Definition::Function(f) => {
                let lets = self.gen_lets(&get_args_names(f));

                src.insert_str(
                    (f.body.first().unwrap().location().start
                        - targeted.definition.location().start) as usize,
                    &format!("\n  {lets}"),
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
            Statement::Use(_) => self.run_use(&src[start..end]),
            Statement::Expression(_) => self.run_expr(&src[start..end]),
            Statement::Assignment(a) => {
                let assign_name: Option<String> = if let Pattern::Assign { name, .. } = &a.pattern {
                    Some(name.into())
                } else {
                    None
                };
                let mut names = vec![];
                assignment_find_names(&a.pattern, &mut names);
                if names.is_empty() {
                    let end = a.value.location().end as usize;
                    self.run_expr(&src[start..end])
                } else {
                    let pattern_end = a.pattern.location().end as usize;
                    let value_start = a.value.location().start as usize;
                    self.run_assignment(
                        assign_name,
                        &src[start..pattern_end],
                        &src[value_start..end],
                        &names,
                    )
                }
            }
            Statement::Assert(_) => self.run_assert(&src[start..end]),
        }
    }

    fn run_type_cmd(&mut self, code: &str) -> Result<(), Error> {
        let mut src = self.build_source();
        self.add_expr(&mut src, code);
        let module = self.compile(&src)?.split_off_first().0;
        let main = &get_function(&module, REPL_MAIN).expect("repl main function");
        println!("{}", type_to_string(&module, &main.return_type));
        Ok(())
    }

    fn run_check(&mut self) -> Result<(), Error> {
        self.compile(&self.build_source()).map(|_| ())
    }

    fn run_assignment(
        &mut self,
        assigned_name: Option<String>,
        pattern: &str,
        value: &str,
        names: &[String],
    ) -> Result<(), Error> {
        let mut src = self.build_source();
        let lets = self.gen_lets(&[]);
        let joined_names = names.join(", ");
        let save_names = names
            .iter()
            .map(|name| format!("repl_save({name})"))
            .collect::<Vec<_>>()
            .join("\n  ");
        let assignment = if let Some(assigned_name) = assigned_name {
            format!("  {pattern} = {value}\n  repl_print({assigned_name})")
        } else {
            format!("  {pattern} as {REPL_MAIN} = {value}\n  repl_print({REPL_MAIN})")
        };
        // FIXME: avoid name collision
        src.push_str(&formatdoc! {"
            pub fn {REPL_MAIN}() {{
              {lets}
              {assignment}
              {save_names}
              #({joined_names})
            }}
            "
        });

        let module = self.compile(&src)?.split_off_first().0;

        self.engine
            .run_main(&module.name, MainFunction::ReplMain, false);

        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, REPL_MAIN).expect("repl main function");
            let types = main.return_type.tuple_types().unwrap();
            assert_eq!(types.len(), names.len());
            for (name, type_) in names.iter().zip(&types) {
                let index = self.var_index;
                let type_ = type_to_string(&module, type_);
                self.vars.insert(name.into(), Variable { index, type_ });
                self.var_index += 1;
            }
        } else {
            // there was an error and the variable was not saved
        }

        Ok(())
    }

    fn run_expr(&mut self, code: &str) -> Result<(), Error> {
        let mut src = self.build_source();
        self.add_expr(&mut src, code);
        let module = self.compile(&src)?.split_off_first().0;
        self.engine
            .run_main(&module.name, MainFunction::ReplMain, false);
        Ok(())
    }

    fn run_assert(&mut self, code: &str) -> Result<(), Error> {
        let mut src = self.build_source();
        let lets = self.gen_lets(&[]);
        src.push_str(&formatdoc! {"
            pub fn {REPL_MAIN}() {{
                {lets}
                {code}
            }}
            "
        });
        let module = self.compile(&src)?.split_off_first().0;
        self.engine
            .run_main(&module.name, MainFunction::ReplMain, false);
        Ok(())
    }

    fn run_import(&mut self, _code: String) -> Result<(), Error> {
        println!("imports are not supported.");
        Ok(())
        // TODO: implement import merge
        // import gleam/string.{append}
        // import gleam/string.{inspect}
        // -> import gleam/string.{append, inspect}
    }

    fn run_const(&mut self, code: String) -> Result<(), Error> {
        // TODO: improve error message for const redefinition
        self.consts.push(code);
        self.run_check()
    }

    fn run_type(&mut self, code: String) -> Result<(), Error> {
        // TODO: improve error message for type redefinition
        self.types.push(code);
        self.run_check()
    }

    fn run_fn(&mut self, name: String, body: String) -> Result<(), Error> {
        self.fns.insert(
            name,
            Function {
                index: self.var_index,
                body,
            },
        );
        self.run_check()
    }

    fn run_use(&mut self, _code: &str) -> Result<(), Error> {
        println!("use statements are not supported outside blocks.");
        Ok(())
    }

    fn add_expr(&self, src: &mut String, expr: &str) {
        let lets = self.gen_lets(&[]);
        src.push_str(&formatdoc! {"
            pub fn {REPL_MAIN}() {{
              {lets}
              repl_print({{
            {expr}
              }})
            }}
            "
        });
    }

    fn add_imports(&self, src: &mut String) {
        if let Some(user) = &self.user_import {
            swriteln!(src, "{user}");
        }
        for import in &self.imports {
            swriteln!(src, "import {import}");
        }
    }

    fn add_consts(&self, src: &mut String) {
        for const_ in &self.consts {
            swriteln!(src, "{const_}");
        }
    }

    fn add_types(&self, src: &mut String) {
        for type_ in &self.types {
            swriteln!(src, "{type_}");
        }
    }

    fn add_fns(&self, src: &mut String) {
        for fun in self.fns.values() {
            swriteln!(src, "{}", fun.body);
        }
    }

    fn gen_lets(&self, exclude: &[String]) -> String {
        let mut lets = String::new();
        for (name, Variable { index, type_ }) in &self.vars {
            let replaced_by_fn = self
                .fns
                .get(name)
                .map(|f| *index < f.index)
                .unwrap_or(false);
            if !exclude.contains(name) && !replaced_by_fn {
                swriteln!(
                    lets,
                    r#"  let {name} = fn () -> {type_} {{ repl_load({index}) }} ()"#
                );
            }
        }
        lets
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

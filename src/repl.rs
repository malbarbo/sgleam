use std::{collections::HashMap, fmt::Write};

use gleam_core::{
    ast::{Definition, Pattern, Statement, TargetedDefinition, UntypedStatement},
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
@external(javascript, "./sgleam_ffi.mjs", "repl_save")
pub fn repl_save(value: a) -> a

@external(javascript, "./sgleam_ffi.mjs", "repl_load")
pub fn repl_load(index: Int) -> a
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
    fns: HashMap<String, String>,
    vars: HashMap<String, Value>,
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
struct Value {
    index: usize,
    type_: String,
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
                    (f.body.first().location().start - targeted.definition.location().start)
                        as usize,
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
            Statement::Assignment(a) => match a.pattern {
                Pattern::Variable { name, .. } => {
                    let end = a.value.location().end as usize;
                    self.run_let(name.as_str(), &src[start..end])
                }
                Pattern::Discard { .. } => {
                    let end = a.value.location().end as usize;
                    self.run_expr(&src[start..end])
                }
                _ => {
                    println!("patterns are not supported in let statements.");
                    Ok(())
                }
            },
            Statement::Assert(_) => {
                println!("assert is not supported.");
                Ok(())
            }
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

    fn run_let(&mut self, name: &str, code: &str) -> Result<(), Error> {
        let mut src = self.build_source();
        let lets = self.gen_lets(&[]);
        // FIXME: avoid name collision
        src.push_str(&formatdoc! {"
            pub fn {REPL_MAIN}() {{
              {lets}
              io.debug(repl_save({{
            {code}
              }}))
            }}
            "
        });

        let module = self.compile(&src)?.split_off_first().0;

        self.engine
            .run_main(&module.name, MainFunction::ReplMain, false);

        if self.engine.has_var(self.var_index) {
            let main = get_function(&module, REPL_MAIN).expect("repl main function");
            let type_ = type_to_string(&module, &main.return_type);
            let index = self.var_index;
            self.vars.insert(name.into(), Value { index, type_ });
            self.var_index += 1;
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

    fn run_fn(&mut self, name: String, code: String) -> Result<(), Error> {
        self.fns.insert(name, code);
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
              io.debug({{
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
        for code in self.fns.values() {
            swriteln!(src, "{code}");
        }
    }

    fn gen_lets(&self, exclude: &[String]) -> String {
        let mut lets = String::new();
        for (name, Value { index, type_ }) in &self.vars {
            if !exclude.contains(name) {
                swriteln!(
                    lets,
                    r#"  let {name} = fn () -> {type_} {{ repl_load({index}) }} ()"#
                );
            }
        }
        lets
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

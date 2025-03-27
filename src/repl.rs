use std::{collections::HashMap, fmt::Write};

use gleam_core::{
    ast::{Definition, Pattern, Statement, TargetedDefinition, UntypedStatement},
    build::Module,
    io::FileSystemWriter,
    Error,
};
use indoc::formatdoc;
use rquickjs::{Array, Context};

use crate::{
    error::{show_error, SgleamError},
    gleam::{compile, get_module, type_to_string, Project},
    javascript::{self, MainFunction},
    parser::{self, ReplItem},
    repl_reader::ReplReader,
    run::get_function,
    swrite, swriteln, GLEAM_MODULES_NAMES,
};

const FNS_REPL: &str = r#"
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
struct Value {
    index: usize,
    type_: String,
}

#[derive(Clone)]
pub struct Repl {
    user_import: Option<String>,
    imports: Vec<String>,
    consts: Vec<String>,
    types: Vec<String>,
    fns: Vec<(String, Option<String>)>,
    vars: HashMap<String, Value>,
    project: Project,
    context: Context,
    type_: bool,
    iter: usize,
    var_index: usize,
}

enum EntryKind {
    Let(String, String),
    Expr(String),
    Other,
}

impl Repl {
    pub fn new(project: Project, user_module: Option<&Module>) -> Result<Repl, SgleamError> {
        let imports = GLEAM_MODULES_NAMES.iter().map(|s| s.to_string()).collect();
        let fs = project.fs.clone();
        Ok(Repl {
            user_import: user_module.map(import_public_types_and_values),
            imports,
            consts: vec![],
            types: vec![],
            fns: vec![],
            vars: HashMap::new(),
            project,
            context: javascript::create_context(fs, Project::out().into())?,
            type_: false,
            iter: 0,
            var_index: 0,
        })
    }

    pub fn run(&mut self) -> Result<(), SgleamError> {
        let reader = ReplReader::new()?;
        for mut input in reader {
            let line_trim = input.trim();
            if line_trim.is_empty() || line_trim.starts_with("//") {
                continue;
            }

            if line_trim == QUIT {
                break;
            }

            if let Some(expr) = line_trim.strip_prefix(TYPE) {
                self.type_ = true;
                input = expr.into();
            } else {
                self.type_ = false;
            }

            let items = parser::parse_repl(&input).map_err(|error| Error::Parse {
                path: format!("/src/repl{}.gleam", self.iter).into(),
                src: input.clone().into(),
                error,
            });

            let items = match items {
                Err(err) => {
                    self.iter += 1;
                    show_error(&SgleamError::Gleam(err));
                    continue;
                }
                Ok(items) => items,
            };

            // FIXME: avoid this clone
            // We clone self so we can rollback if the execution fail
            let repl = (*self).clone();

            for item in items {
                self.iter += 1;
                let result = match item {
                    ReplItem::ReplDefinition(t) => self.run_definition(t, &input),
                    ReplItem::ReplStatement(s) => self.run_statement(s, &input),
                };

                if let Err(err) = result {
                    show_error(&SgleamError::Gleam(err));
                    *self = repl;
                    break;
                }
            }
        }
        Ok(())
    }

    fn run_code(&mut self, kind: EntryKind) -> Result<(), Error> {
        let mut src = String::new();
        src.push_str(FNS_REPL);
        self.add_imports(&mut src);
        self.add_consts(&mut src);
        self.add_types(&mut src);
        self.add_fns(&mut src);

        match &kind {
            EntryKind::Let(_, expr) => {
                // FIXME: can we generate code that generates better error messagens?
                // Examples of entries that generates poor errors
                // "pub "
                // "let"
                let lets = self.get_lets(&[]);
                src.push_str(&formatdoc! {"
                    pub fn main() {{
                    {lets}
                      io.debug(repl_save({{
                        {expr}
                      }}))
                    }}
                    "
                });
            }
            EntryKind::Expr(expr) => {
                let lets = self.get_lets(&[]);
                src.push_str(&formatdoc! {"
                    pub fn main() {{
                      {lets}
                      io.debug({{
                        {expr}
                      }})
                    }}
                    "
                });
            }
            _ => {
                // main function is not needed
            }
        }

        let iter = self.iter;
        let module_name = format!("repl{iter}");
        let file = format!("{module_name}.gleam");

        // TODO: add an option to show the generated code
        self.project.write_source(&file, &src);

        let result = compile(&mut self.project, true);

        if let Ok(modules) = &result {
            let module = get_module(modules, &module_name).expect("The repl module");
            if let EntryKind::Let(_, _) | EntryKind::Expr(_) = &kind {
                // TODO: change the output for functions (show the function type)
                // TODO: should we show the type of all expressions? See haskell, elm, ocaml, roc.
                if self.type_ {
                    let type_ = get_function(module, "main")
                        .expect("main function")
                        .return_type
                        .clone();
                    println!("{}", type_to_string(module, &type_));
                } else {
                    javascript::run_main(&self.context, &module_name, MainFunction::Main, false);
                }
            } else {
                // Nothing to run, was a definition (type, const, import or fn)
            }

            if let EntryKind::Let(name, _) = &kind {
                if self.try_save_var(name, self.var_index, module) {
                    self.var_index += 1;
                }
            }
        }

        // TODO: Can we remove the file after the compilation?
        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

        result.map(|_| ())
    }

    fn run_definition(&mut self, targeted: TargetedDefinition, src: &str) -> Result<(), Error> {
        let start = targeted.definition.location().start as usize;
        let end = targeted.definition.location().end as usize;

        match targeted.definition {
            Definition::Import(_) => {
                let code = String::from(&src[start..end]);
                self.run_import(code)
            }
            Definition::CustomType(t) => {
                let end = t.end_position as usize;
                let code = String::from(&src[start..end]);
                self.run_type(code)
            }
            Definition::TypeAlias(_) => {
                let code = String::from(&src[start..end]);
                self.run_type(code)
            }
            Definition::ModuleConstant(c) => {
                let end = c.value.location().end as usize;
                let code = String::from(&src[start..end]);
                self.run_const(code)
            }
            Definition::Function(f) => {
                let end = f.end_position as usize;
                let mut code = String::from(&src[start..end]);

                let args: Vec<String> = f
                    .arguments
                    .into_iter()
                    .filter_map(|arg| arg.names.get_variable_name().map(String::from))
                    .collect();

                let lets = self.get_lets(&args);

                code.insert_str(
                    f.body.first().location().start as usize - start,
                    &format!("\n  {lets}"),
                );

                let name: Option<String> = f.name.map(|spanned| spanned.1.into());
                self.run_fn(code, name)
            }
        }
    }

    fn run_statement(&mut self, statement: UntypedStatement, src: &str) -> Result<(), Error> {
        let start = statement.location().start as usize;
        let end = statement.location().end as usize;

        match statement {
            Statement::Use(_) => {
                let code = String::from(&src[start..end]);
                self.run_use(code)
            }
            Statement::Expression(_) => {
                let code = String::from(&src[start..end]);
                self.run_expr(code)
            }
            Statement::Assignment(a) => match a.pattern {
                Pattern::Variable { name, .. } => {
                    let end = a.value.location().end as usize;
                    let code = String::from(&src[start..end]);
                    self.run_let(name.into(), code)
                }
                Pattern::Discard { .. } => {
                    let end = a.value.location().end as usize;
                    let code = String::from(&src[start..end]);
                    self.run_expr(code)
                }
                _ => {
                    println!("patterns are not supported in let statements.");
                    Ok(())
                }
            },
        }
    }

    fn run_import(&mut self, _code: String) -> Result<(), Error> {
        println!("imports are not supported.");
        Ok(())
        // TODO: implement import merge
        // import gleam/string.{append}
        // import gleam/string.{inspect}
        // -> import gleam/string.{append, inspect}
        // let new_import = code.trim().strip_prefix("import ").unwrap_or("");
        // self.imports.push(new_import.into());
        // self.run_code(EntryKind::Other)
    }

    fn run_const(&mut self, code: String) -> Result<(), Error> {
        // TODO: improve error message for const redefinition
        self.consts.push(code);
        self.run_code(EntryKind::Other)
    }

    fn run_type(&mut self, code: String) -> Result<(), Error> {
        // TODO: improve error message for type redefinition
        self.types.push(code);
        self.run_code(EntryKind::Other)
    }

    fn run_fn(&mut self, code: String, name: Option<String>) -> Result<(), Error> {
        for (i, (_, other_name)) in self.fns.iter().enumerate() {
            if !other_name.is_none() && *other_name == name {
                let _ = self.fns.remove(i);
                break;
            }
        }

        self.fns.push((code, name));
        self.run_code(EntryKind::Other)
    }

    fn run_let(&mut self, name: String, code: String) -> Result<(), Error> {
        self.run_code(EntryKind::Let(name, code))
    }

    fn run_use(&mut self, _code: String) -> Result<(), Error> {
        println!("use statements are not supported outside blocks.");
        Ok(())
    }

    fn run_expr(&mut self, code: String) -> Result<(), Error> {
        self.run_code(EntryKind::Expr(code))
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
        for fn_ in &self.fns {
            let (code, _) = fn_;
            swriteln!(src, "{code}");
        }
    }

    fn get_lets(&mut self, exclude: &[String]) -> String {
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

    fn try_save_var(&mut self, name: &str, index: usize, module: &Module) -> bool {
        if !self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Array>("repl_vars")
                .map(|a| index < a.len())
                .unwrap_or(false)
        }) {
            // the expression crashed and repl_save was not called
            return false;
        }

        let return_type = module
            .ast
            .definitions
            .iter()
            .filter_map(|d| d.main_function())
            .next()
            .expect("The main function")
            .return_type
            .clone();

        self.vars.insert(
            name.into(),
            Value {
                index,
                type_: type_to_string(module, &return_type),
            },
        );

        true
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

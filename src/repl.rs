use std::{collections::HashMap, fmt::Write};

use gleam_core::{build::Module, io::FileSystemWriter, Error};
use indoc::formatdoc;
use rquickjs::{Array, Context};

use crate::{
    error::{show_error, SgleamError},
    gleam::{compile, get_module, type_to_string, Project},
    javascript::{self, MainKind},
    repl_reader::ReplReader,
    run::get_main,
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
pub struct Repl {
    user_import: Option<String>,
    imports: Vec<String>,
    consts: Vec<String>,
    types: Vec<String>,
    fns: Vec<String>,
    vars: HashMap<String, (usize, String)>,
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
        let editor = ReplReader::new()?;
        for mut code in editor {
            let code_trim = code.trim();
            if code_trim.is_empty() || code_trim.starts_with("//") {
                continue;
            }

            if code_trim == QUIT {
                break;
            }

            if let Some(expr) = code_trim.strip_prefix(TYPE) {
                self.type_ = true;
                code = expr.into();
            } else {
                self.type_ = false;
            }

            self.iter += 1;

            // FIXME: avoid this clone
            // We clone self so we can rollback if the execution fail
            let mut repl = (*self).clone();

            let code_no_pub = code.trim_start().strip_prefix("pub ").unwrap_or(&code);
            let pub_code = format!("pub {code_no_pub}");
            let result = match (code_no_pub.split_whitespace().next(), self.type_) {
                (Some("import"), false) => repl.run_import(code),
                (Some("const"), false) => repl.run_const(pub_code),
                (Some("type"), false) => repl.run_type(pub_code),
                (Some("let"), false) => repl.run_let(code),
                (Some("fn"), false) => repl.run_fn(pub_code),
                _ => repl.run_expr(code),
            };

            if let Err(err) = result {
                show_error(&err.into());
            } else {
                // rollback
                *self = repl;
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

        let ret = if self.type_ { "" } else { "Nil" };

        match &kind {
            EntryKind::Let(_, expr) => {
                // FIXME: can we generate code that generates better error messagens?
                // Examples of entries that generates poor errors
                // "pub "
                // "let"
                let lets = self.get_lets();
                src.push_str(&formatdoc! {"
                    pub fn main() {{
                    {lets}
                      io.debug(repl_save({{
                        {expr}
                      }}))
                      {ret}
                    }}
                    "
                });
            }
            EntryKind::Expr(expr) => {
                let lets = self.get_lets();
                src.push_str(&formatdoc! {"
                    pub fn main() {{
                      {lets}
                      io.debug({{
                        {expr}
                      }})
                      {ret}
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
                if self.type_ {
                    let type_ = get_main(module).expect("main function").return_type.clone();
                    println!("{}", type_to_string(type_));
                } else {
                    javascript::run_main(&self.context, MainKind::Nil, &module_name);
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

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

        result.map(|_| ())
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

    fn run_fn(&mut self, code: String) -> Result<(), Error> {
        if let Some((pub_fn_name, code)) = code.split_once('(') {
            if let Some(name) = pub_fn_name.strip_prefix("pub fn").map(str::trim) {
                if !name.contains(char::is_whitespace) {
                    // TODO: check if the compiler erros are ok
                    return self.run_let(format!("let {name} = fn({code}"));
                }
            }
        }
        // We could not transforme the code to a let expression, so it can be an anonymous function
        self.run_code(EntryKind::Expr(code))
    }

    fn run_let(&mut self, code: String) -> Result<(), Error> {
        if let Some(name) = code
            .trim()
            .strip_prefix("let")
            .and_then(|s| s.split_once('=').map(|s| s.0))
            .map(|s| s.split_once(':').map(|s| s.0).unwrap_or(s))
            .map(str::trim)
        {
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return self.run_code(EntryKind::Let(name.into(), code));
            } else {
                println!("Only let with single names are supported.");
                return Ok(());
            }
        }
        // We could not get the binding name, so we run it to fail
        self.run_code(EntryKind::Expr(code))
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
            swriteln!(src, "{fn_}");
        }
    }

    fn get_lets(&mut self) -> String {
        let mut lets = String::new();
        for (name, (index, ty)) in &self.vars {
            swriteln!(lets, r#"  let {name}: {ty} = repl_load({index})"#);
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

        self.vars
            .insert(name.into(), (index, type_to_string(return_type)));

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

use camino::Utf8PathBuf;
use clap::{
    arg,
    builder::{styling, Styles},
    command, Parser,
};
use gleam_core::{
    build::Module,
    io::FileSystemWriter,
    type_::{self, Type, TypeVar},
    Error,
};
use im::HashMap;
use rquickjs::{Context, Value};
use sgleam::{
    format,
    gleam::{
        compile, get_main_function, get_module, show_gleam_error, to_error_nonutf8_path, Project,
    },
    javascript::{create_js_context, run_js},
    logger, panic,
    repl::ReplReader,
    STACK_SIZE,
};
use std::{fmt::Write, path::PathBuf, process::exit, sync::Arc, thread};
use vec1::vec1;

/// The student version of gleam.
#[derive(Parser)]
#[command(
    about,
    styles = Styles::styled()
        .header(styling::AnsiColor::Yellow.on_default())
        .usage(styling::AnsiColor::Yellow.on_default())
        .literal(styling::AnsiColor::Green.on_default())
)]
struct Cli {
    /// Go to iterative mode.
    #[arg(short, group = "cmd")]
    interative: bool,
    /// Run tests.
    #[arg(short, group = "cmd")]
    test: bool,
    /// Format source code.
    #[arg(short, group = "cmd")]
    format: bool,
    /// Print version.
    #[arg(short, long)]
    version: bool,
    /// The program file.
    // TODO: allow multiple files
    path: Option<PathBuf>,
}

fn main() {
    panic::add_handler();
    logger::initialise_logger();
    // Error is handled by the panic hook.
    let _ = thread::Builder::new()
        .stack_size(STACK_SIZE)
        .name("run".into())
        .spawn(|| {
            if let Err(err) = run() {
                show_gleam_error(err);
            }
        })
        .expect("Create the run thread")
        .join();
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    // TODO: include quickjs version
    if cli.version {
        println!("{}", sgleam::version());
        return Ok(());
    }

    let path = if let Some(path) = cli.path {
        path
    } else {
        Repl::new(Project::default(), None).run();
        return Ok(());
    };

    // TODO: create a method validade_input
    let input = Utf8PathBuf::from_path_buf(path).map_err(to_error_nonutf8_path)?;
    if !input.is_file() {
        eprintln!("{input}: does not exist or is not a file.");
        exit(1);
    }

    let main_module = input.file_stem().unwrap_or("");
    if input.extension() != Some("gleam") || main_module.is_empty() {
        eprintln!("{input}: is not a valid gleam file.");
        exit(1);
    }

    if main_module == "sgleam" {
        return Err(Error::Type {
            path: input,
            src: "".into(),
            errors: vec1![type_::Error::ReservedModuleName {
                name: "sgleam".into(),
            }],
        });
    }

    if cli.format {
        // TODO: allow read the file from stdin
        format::run(false, false, vec![input.into()])?;
        return Ok(());
    }

    let mut project = Project::default();
    project.copy_to_source(&input)?;

    let modules = compile(&mut project, false)?;

    if let Some(module) = get_module(&modules, main_module) {
        if cli.interative {
            Repl::new(project, Some(module)).run();
        } else {
            if !cli.test {
                let _mainf = get_main_function(module)?;
            }
            let context = create_js_context(project.fs.clone(), Project::out().into());
            let source = main_js_script(main_module, cli.test);
            run_js(&context, source);
        }
    } else {
        // The compiler ignored the file because of the name and printed an warning.
    }

    Ok(())
}

const GLEAM_MODULES_NAMES: &[&str] = &[
    "gleam/bit_array",
    "gleam/bool",
    "gleam/bytes_builder",
    "gleam/dict",
    "gleam/dynamic",
    "gleam/float",
    "gleam/function",
    "gleam/int",
    "gleam/io",
    "gleam/iterator",
    "gleam/list",
    "gleam/option",
    "gleam/order",
    "gleam/pair",
    "gleam/queue",
    "gleam/regex",
    "gleam/result",
    "gleam/set",
    "gleam/string",
    "gleam/string_builder",
    "gleam/uri",
];

const FN_MAIN_NIL: &str = "
pub fn main() {
  Nil
}
";

const FN_GET_GLOBAL: &str = r#"
@external(javascript, "./sgleam_ffi.mjs", "get_global")
pub fn get_global(name: String) -> a
"#;

#[derive(Clone)]
struct Repl {
    user_import: Option<String>,
    imports: Vec<String>,
    consts: Vec<String>,
    types: Vec<String>,
    fns: Vec<String>,
    vars: HashMap<String, (usize, String)>,
    project: Project,
    context: Context,
    iter: usize,
}

enum EntryKind {
    // FIXME: add the binding name
    Let(String),
    Expr(String),
    Other,
}

impl Repl {
    fn new(project: Project, user_module: Option<&Module>) -> Repl {
        let imports = GLEAM_MODULES_NAMES.iter().map(|s| s.to_string()).collect();
        let fs = project.fs.clone();
        Repl {
            user_import: user_module.map(import_public_types_and_values),
            imports,
            consts: vec![],
            types: vec![],
            fns: vec![],
            vars: HashMap::new(),
            project,
            context: create_js_context(fs, Project::out().into()),
            iter: 0,
        }
    }

    fn run(&mut self) {
        let editor = ReplReader::new().expect("Create the reader for repl");
        for code in editor.filter(|s| !s.trim().is_empty() && !s.trim().starts_with("//")) {
            self.iter += 1;

            // FIXME: avoid this clone
            // We clone self so we can rollback if the execution fail
            let mut repl = (*self).clone();

            let code_no_pub = code.trim_start().strip_prefix("pub ").unwrap_or(&code);
            let pub_code = format!("pub {code_no_pub}");
            let result = match code_no_pub.split_whitespace().next() {
                Some("import") => repl.run_import(code),
                Some("const") => repl.run_const(pub_code),
                Some("type") => repl.run_type(pub_code),
                Some("let") => repl.run_let(code),
                Some("fn") => repl.run_fn(pub_code),
                _ => repl.run_expr(code),
            };

            if let Err(err) = result {
                show_gleam_error(err);
            } else {
                // rollback
                *self = repl;
            }
        }
    }

    fn run_code(&mut self, kind: EntryKind) -> Result<(), Error> {
        let mut src = String::new();
        src.push_str(FN_GET_GLOBAL);
        self.add_imports(&mut src);
        self.add_consts(&mut src);
        self.add_types(&mut src);
        self.add_fns(&mut src);

        let main = if let EntryKind::Let(expr) | EntryKind::Expr(expr) = &kind {
            // FIXME: can we generate code that generates better error messagens?
            // Examples of entries that generates poor errors
            // "pub "
            // "let"
            // TODO: improve how the results are printed.
            // Can we show function names and signature?
            // > fun add1(a) { int.to_float(a) +. 1.0 }
            // fn (Int) -> Float // add
            let lets = self.get_lets();
            format!(
                "
pub fn main() {{
{lets}
  io.debug({{
    {expr}
  }})
}}
"
            )
        } else {
            FN_MAIN_NIL.into()
        };
        src.push('\n');
        src.push_str(&main);

        let iter = self.iter;
        let module_name = format!("repl{iter}");
        let file = format!("{module_name}.gleam");

        // TODO: add an option to show the generated code
        self.project.write_source(&file, &src);

        let result = compile(&mut self.project, true);

        if let Ok(modules) = &result {
            if let EntryKind::Let(expr) = kind {
                run_js(&self.context, main_let(&module_name, iter));
                if self.has_var(iter) {
                    let name = expr
                        .trim()
                        .strip_prefix("let")
                        .and_then(|s| s.split('=').next())
                        .map(str::trim)
                        .expect("A var name");
                    self.save_var(
                        name,
                        iter,
                        get_module(modules, &module_name).expect("The repl module"),
                    );
                }
            } else {
                run_js(&self.context, main_js_script(&module_name, false));
            }
        }

        self.project
            .fs
            .delete_file(&Project::source().join(file))
            .expect("To delete repl file");

        result.map(|_| ())
    }

    fn run_import(&mut self, code: String) -> Result<(), Error> {
        // TODO: implement import merge
        // import gleam/string.{append}
        // import gleam/string.{inspect}
        // -> import gleam/string.{append, inspect}
        let new_import = code.trim().strip_prefix("import ").unwrap_or("");
        self.imports.push(new_import.into());
        self.run_code(EntryKind::Other)
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
        // We could not transformed the code to a let expression, so we run it to fail
        self.fns.push(code);
        self.run_code(EntryKind::Other)
    }

    fn run_let(&mut self, code: String) -> Result<(), Error> {
        if let Some((name, _)) = code
            .trim()
            .strip_prefix("let")
            .and_then(|s| s.split_once('='))
        {
            if name.trim().chars().all(|c| c.is_alphanumeric() || c == '_') {
                return self.run_code(EntryKind::Let(code));
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
            src.push_str(user);
            src.push('\n');
        }
        for import in &self.imports {
            let _ = writeln!(src, "import {import}");
        }
    }

    fn add_consts(&self, src: &mut String) {
        src.push_str(&self.consts.join("\n"));
        src.push('\n');
    }

    fn add_types(&self, src: &mut String) {
        src.push_str(&self.types.join("\n"));
        src.push('\n');
    }

    fn add_fns(&self, src: &mut String) {
        src.push_str(&self.fns.join("\n"));
    }

    fn get_lets(&mut self) -> String {
        let mut lets = String::new();
        for (name, (iter, type_)) in &self.vars {
            let _ = writeln!(
                lets,
                r#"  let {name}: {type_} = get_global("repl_var_{iter}")"#
            );
        }
        lets
    }

    fn has_var(&self, iter: usize) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Value>(format!("repl_var_{iter}"))
                .map(|v| !v.is_undefined())
                .unwrap_or(false)
        })
    }

    fn save_var(&mut self, name: &str, iter: usize, module: &Module) {
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
            (iter, type_to_string(return_type, &mut vec![])),
        );
    }
}

// FIXME: check in the lsp module how the action "Add type annotation" works
fn type_to_string(type_: Arc<Type>, unbounds: &mut Vec<Arc<Type>>) -> String {
    if let Some((_, return_type)) = type_.named_type_name() {
        return return_type.into();
    }

    if let Some((args, return_type)) = type_.fn_types() {
        let args = args
            .iter()
            .map(|arg| type_to_string(arg.clone(), unbounds))
            .collect::<Vec<_>>()
            .join(",");
        let return_type = type_to_string(return_type, unbounds);
        return format!("fn({args}) -> {return_type}");
    }

    if let Some(types_) = type_.tuple_types() {
        let types_ = types_
            .iter()
            .map(|type_| type_to_string(type_.clone(), unbounds))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("#({types_})");
    }

    if let Type::Var { type_: t } = &*type_ {
        let type_ = if let TypeVar::Link { type_ } = t.borrow().clone() {
            type_
        } else {
            type_.clone()
        };

        let pos = unbounds
            .iter()
            .position(|t| *t == type_)
            .unwrap_or_else(|| {
                let pos = unbounds.len();
                unbounds.push(type_);
                pos
            });
        return char::from_u32('a' as u32 + pos as u32)
            .expect("A char from u32")
            .into();
    }

    panic!("Unknow type\n{:#?}", type_);
}

fn import_public_types_and_values(module: &Module) -> String {
    let mut import = String::new();
    let name = &module.name;
    let _ = write!(&mut import, "import {name}.{{");
    for type_ in module.ast.type_info.public_type_names() {
        let _ = write!(&mut import, "type {type_},");
    }
    for value in module.ast.type_info.public_value_names() {
        let _ = write!(&mut import, "{value},");
    }
    import.push('}');
    import
}

fn main_js_script(module: &str, test: bool) -> String {
    if !test {
        // FIXME: use indoc crate (formatdoc macro) to write these strings
        format!(
            r#"
import {{ try_main }} from "./sgleam_ffi.mjs";
import {{ main }} from "./{module}.mjs";
try_main(main);
"#
        )
    } else {
        format!(
            r#"
import {{ run_tests }} from "./sgleam_ffi.mjs";
import * as {module} from "./{module}.mjs";
run_tests({module});
"#
        )
    }
}

fn main_let(module: &str, iter: usize) -> String {
    format!(
        r#"
import {{ try_main }} from "./sgleam_ffi.mjs";
import {{ main }} from "./{module}.mjs";
let r = try_main(main);
if (r !== undefined) {{
    globalThis.repl_var_{iter} = r;
}}"#
    )
}

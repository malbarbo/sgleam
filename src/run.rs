use camino::Utf8Path;
use gleam_core::{
    ast::{TypedDefinition, TypedFunction},
    build::{Module, Target},
};
use std::process::exit;

use crate::{
    error::SgleamError,
    gleam::{compile, fn_type_to_string, get_module, type_to_string, Project},
    javascript::{self, MainInput},
    repl::{welcome_message, Repl},
};

pub fn run_interative(path: Option<&String>, quiet: bool) -> Result<(), SgleamError> {
    if !quiet {
        print!("{}", welcome_message());
    }

    let mut project = Project::default();

    let input = path.map(Utf8Path::new);
    if let Some(input) = input {
        if !validade_path(input) {
            exit(1);
        }
        project.copy_to_source(input)?;
    }

    let modules = compile(&mut project, false)?;
    let module = input
        .and_then(|input| input.file_stem())
        .and_then(|module_name| get_module(&modules, module_name));

    Repl::new(project, module)?.run()?;

    Ok(())
}

pub fn run_main(path: &str) -> Result<(), SgleamError> {
    let path = Utf8Path::new(path);
    if !validade_path(path) {
        exit(1);
    }

    let mut project = Project::default();
    project.copy_to_source(path)?;

    let modules = compile(&mut project, false)?;

    if let Some(module) = path.file_stem().and_then(|name| get_module(&modules, name)) {
        javascript::run_main(
            &javascript::create_context(project.fs.clone(), Project::out().into())?,
            &module.name,
            get_main_kind(module)?,
            true,
        );
    } else {
        // The compiler ignored the file because of the name and printed a warning.
    }

    Ok(())
}

pub fn get_main(module: &Module) -> Result<&TypedFunction, gleam_core::Error> {
    module
        .ast
        .definitions
        .iter()
        .find_map(|def| match def {
            TypedDefinition::Function(f)
                if f.name.as_ref().map(|s| s.1.as_str()) == Some("main") =>
            {
                Some(f)
            }
            _ => None,
        })
        .ok_or_else(|| gleam_core::Error::ModuleDoesNotHaveMainFunction {
            module: module.name.clone(),
        })
}

pub fn get_main_kind(module: &Module) -> Result<MainInput, SgleamError> {
    let main = get_main(module)?;

    if !main.implementations.supports(Target::JavaScript) {
        return Err(gleam_core::Error::MainFunctionDoesNotSupportTarget {
            module: module.name.clone(),
            target: Target::JavaScript,
        }
        .into());
    }

    match &main.arguments[..] {
        [] => Ok(MainInput::Nothing),
        // TODO: make the signatures generic, also in show_error
        [arg] if type_to_string(arg.type_.clone()) == "String" => Ok(MainInput::Stdin),
        [arg] if type_to_string(arg.type_.clone()) == "List(String)" => Ok(MainInput::StdinLines),
        _ => Err(SgleamError::InvalidMain {
            module: module.name.clone(),
            signature: {
                let args = main
                    .arguments
                    .iter()
                    .map(|arg| arg.type_.clone())
                    .collect::<Vec<_>>();
                fn_type_to_string(&args[..], main.return_type.clone()).into()
            },
        }),
    }
}

pub fn run_check_or_test(paths: &[String], test: bool) -> Result<(), SgleamError> {
    let mut project = Project::default();

    for path in paths.iter().map(Utf8Path::new).filter(|p| validade_path(p)) {
        project.copy_to_source(path)?;
    }

    let modules = compile(&mut project, false)?;

    if test {
        let modules: Vec<_> = modules
            .iter()
            .map(|m| m.name.as_str())
            .filter(|name| !name.starts_with("gleam/") && !name.starts_with("sgleam/"))
            .collect();
        javascript::run_tests(
            &javascript::create_context(project.fs.clone(), Project::out().into())?,
            &modules,
        );
    }

    Ok(())
}

fn validade_path(path: &Utf8Path) -> bool {
    if !path.is_file() {
        eprintln!("{path}: does not exist or is not a file.");
        return false;
    }

    let steam = path.file_stem().unwrap_or("");
    if path.extension() != Some("gleam") || steam.is_empty() {
        eprintln!("{path}: is not a valid gleam file.");
        return false;
    }

    if steam == "gleam" || steam == "sgleam" {
        eprintln!("{steam}: is a reserved module name.");
        return false;
    }

    true
}

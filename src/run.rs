use camino::Utf8Path;
use gleam_core::Error;
use std::process::exit;

use crate::{
    gleam::{compile, get_main_function, get_module, Project},
    javascript,
    repl::{welcome_message, Repl},
};

pub fn run_interative(path: Option<&String>, quiet: bool) -> Result<(), Error> {
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

    Repl::new(project, module).run();

    Ok(())
}

pub fn run_main(path: &str) -> Result<(), Error> {
    let path = Utf8Path::new(path);
    if !validade_path(path) {
        exit(1);
    }

    let mut project = Project::default();
    project.copy_to_source(path)?;

    let modules = compile(&mut project, false)?;

    if let Some(module) = path.file_stem().and_then(|name| get_module(&modules, name)) {
        let _mainf = get_main_function(module)?;
        javascript::run_main(
            &javascript::create_context(project.fs.clone(), Project::out().into()),
            &module.name,
        );
    } else {
        // The compiler ignored the file because of the name and printed a warning.
    }

    Ok(())
}

pub fn run_check_or_test(paths: &[String], test: bool) -> Result<(), Error> {
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
            &javascript::create_context(project.fs.clone(), Project::out().into()),
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

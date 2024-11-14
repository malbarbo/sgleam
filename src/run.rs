use camino::Utf8Path;
use gleam_core::Error;
use indoc::formatdoc;
use std::{fmt::Write, process::exit};

use crate::{
    gleam::{compile, get_main_function, get_module, Project},
    javascript::{create_js_context, run_js},
    repl::{welcome_message, Repl},
    swriteln,
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
        run_js(
            &create_js_context(project.fs.clone(), Project::out().into()),
            js_main(&module.name),
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
        run_js(
            &create_js_context(project.fs.clone(), Project::out().into()),
            js_main_test(&modules),
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

fn js_main_test(modules: &[&str]) -> String {
    let mut src = String::new();
    swriteln!(
        &mut src,
        r#"import {{ run_tests }} from "./sgleam_ffi.mjs";"#
    );
    for module in modules {
        swriteln!(&mut src, r#"import * as {module} from "./{module}.mjs";"#);
    }
    let names = modules.join(", ");
    swriteln!(&mut src, "run_tests([{names}], {modules:#?});");
    src
}

fn js_main(module: &str) -> String {
    formatdoc! {r#"
        import {{ try_main }} from "./sgleam_ffi.mjs";
        import {{ main }} from "./{module}.mjs";
        try_main(main);
    "#}
}

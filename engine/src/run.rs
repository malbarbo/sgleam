use camino::{Utf8Path, Utf8PathBuf};

use gleam_core::{
    ast::TypedFunction,
    build::{Module, Origin, Target},
    type_,
};

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{fn_type_to_string, get_module, Project},
};

use crate::quickjs::QuickJsEngine as JsEngine;

const SGLEAM_SMAIN: &str = "smain";

pub fn run_main(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, paths)?;
    let name = paths[0].with_extension("");
    let name = name.as_str().replace('\\', "/");

    if let Some(module) = get_module(&modules, &name) {
        let main = get_main(module)?;
        let show_output = main != MainFunction::Main;
        JsEngine::new(project.fs.clone()).run_main(&module.name, main, show_output)?;
    } else {
        // The compiler ignored the file because of the name and printed a warning.
    }

    Ok(())
}

pub fn run_check(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    copy_files_and_build(&mut project, paths)?;
    Ok(())
}

pub fn run_test(user_files: &[Utf8PathBuf], paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, paths)?;
    let modules: Vec<_> = modules
        .iter()
        .filter_map(|module| {
            let path = module
                .input_path
                .strip_prefix("/src/")
                .unwrap_or(Utf8Path::new(""))
                .to_owned();
            if user_files.contains(&path) {
                Some(module.name.as_str())
            } else {
                None
            }
        })
        .collect();

    JsEngine::new(project.fs.clone()).run_tests(&modules)?;
    Ok(())
}

pub fn get_function<'a>(module: &'a Module, name: &str) -> Option<&'a TypedFunction> {
    module
        .ast
        .definitions
        .functions
        .iter()
        .find(|f| f.name.as_ref().map(|s| s.1.as_str()) == Some(name))
}

pub fn get_main(module: &Module) -> Result<MainFunction, SgleamError> {
    match get_smain(module) {
        r @ Ok(_) | r @ Err(SgleamError::InvalidSMain { .. }) => r,
        _ => Ok(module
            .ast
            .type_info
            .get_main_function(Target::JavaScript)
            .map(|_| MainFunction::Main)?),
    }
}

pub fn get_smain(module: &Module) -> Result<MainFunction, SgleamError> {
    let smain = get_function(module, SGLEAM_SMAIN).ok_or_else(|| {
        gleam_core::Error::ModuleDoesNotHaveMainFunction {
            module: module.name.clone(),
            origin: Origin::Src,
        }
    })?;

    if !smain.implementations.supports(Target::JavaScript) {
        return Err(gleam_core::Error::MainFunctionDoesNotSupportTarget {
            module: module.name.clone(),
            target: Target::JavaScript,
        }
        .into());
    }

    let string_type = type_::string();
    let list_string_type = type_::list(type_::string());
    match &smain.arguments[..] {
        [] => Ok(MainFunction::Smain),
        [arg] if arg.type_.same_as(&string_type) => Ok(MainFunction::SmainStdin),
        [arg] if arg.type_.same_as(&list_string_type) => Ok(MainFunction::SmainStdinLines),
        _ => Err(SgleamError::InvalidSMain {
            module: module.name.clone(),
            signature: {
                let args = smain
                    .arguments
                    .iter()
                    .map(|arg| arg.type_.clone())
                    .collect::<Vec<_>>();
                fn_type_to_string(module, &args[..], smain.return_type.clone()).into()
            },
        }),
    }
}

pub fn copy_files_and_build(
    project: &mut Project,
    paths: &[Utf8PathBuf],
) -> Result<Vec<Module>, gleam_core::Error> {
    for path in paths.iter().filter(|p| validate_path(p)) {
        project.copy_file_to_source(path)?;
    }
    let mut modules = project.compile(false)?;
    modules
        .retain(|module| !module.name.starts_with("gleam/") && !module.name.starts_with("sgleam/"));
    Ok(modules)
}

fn validate_path(path: &Utf8Path) -> bool {
    let stem = path.file_stem().unwrap_or("");
    if path.extension() != Some("gleam") || stem.is_empty() {
        crate::quickjs::write_stderr(&format!("Ignoring `{path}`: is not a valid gleam file.\n"));
        return false;
    }

    if stem == "gleam" || stem == "sgleam" {
        crate::quickjs::write_stderr(&format!(
            "Ignoring `{path}`: `{stem}` is a reserved module name.\n"
        ));
        return false;
    }

    true
}

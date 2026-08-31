use camino::Utf8PathBuf;

use gleam_core::{
    build::{Module, Origin, Target},
    type_,
};

use crate::{
    engine::{Engine, MainFunction, SMAIN},
    error::SgleamError,
    gleam::{Project, copy_files_and_build, fn_type_to_string, get_function},
};

// Running a program is what picks a runtime. Nothing above this module names
// one.
use crate::quickjs::QuickJsEngine as JsEngine;

pub fn run_main(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    // The first path is the program, and the rest are what it imports.
    let program = paths.first().expect("a path to run");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: program.clone(),
    })?;

    let main = get_main(module)?;
    JsEngine::new(project.fs.clone())?.run_main(&module.name, main)?;

    Ok(())
}

pub fn run_check(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    copy_files_and_build(&mut project, paths)?;
    Ok(())
}

pub fn run_test(user_files: &[Utf8PathBuf], paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let user_modules: Vec<_> = paths
        .iter()
        .enumerate()
        .filter(|(_, path)| user_files.contains(path))
        .filter_map(|(index, _)| built.module(index))
        .map(|module| module.name.as_str())
        .collect();

    JsEngine::new(project.fs.clone())?.run_tests(&user_modules)?;
    Ok(())
}

fn get_main(module: &Module) -> Result<MainFunction, SgleamError> {
    match get_smain(module) {
        r @ Ok(_) | r @ Err(SgleamError::InvalidSMain { .. }) => r,
        _ => Ok(module
            .ast
            .type_info
            .get_main_function(Target::JavaScript)
            .map(|_| MainFunction::Main)?),
    }
}

fn get_smain(module: &Module) -> Result<MainFunction, SgleamError> {
    let smain = get_function(module, SMAIN).ok_or_else(|| {
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

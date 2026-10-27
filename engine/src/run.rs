use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};

use gleam_core::{
    build::{Module, Origin, Target},
    type_::{self, Type},
};

use crate::{
    engine::{Engine, MainFunction, SMAIN},
    error::SgleamError,
    gleam::{Project, copy_files_and_build, fn_type_to_string, get_function},
    host::HostOptions,
};

// Running a program is what picks a runtime. Nothing above this module names
// one.
use crate::quickjs::QuickJsEngine as JsEngine;

pub fn run_main(paths: &[Utf8PathBuf], options: HostOptions) -> Result<(), SgleamError> {
    // The first path is the program, and the rest are what it imports.
    let program = paths.first().expect("a path to run");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: program.clone(),
    })?;
    run_module(&project, module, program, options)
}

/// Runs the `main` or the `smain` of `module`, which `project` built. An
/// error names the module by `path`.
pub fn run_module(
    project: &Project,
    module: &Module,
    path: &Utf8Path,
    options: HostOptions,
) -> Result<(), SgleamError> {
    let main = get_main(module).map_err(|err| match err {
        SgleamError::Gleam(gleam_core::Error::ModuleDoesNotHaveMainFunction { module, .. }) => {
            SgleamError::NoMainFunction {
                module,
                path: path.into(),
            }
        }
        err => err,
    })?;
    JsEngine::new(project.fs.clone(), options)?.run_main(&module.name, main)?;

    Ok(())
}

pub fn run_check(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    copy_files_and_build(&mut project, paths)?;
    Ok(())
}

pub fn run_test(
    user_files: &[Utf8PathBuf],
    paths: &[Utf8PathBuf],
    options: HostOptions,
) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let user_modules: Vec<_> = paths
        .iter()
        .enumerate()
        .filter(|(_, path)| user_files.contains(path))
        .filter_map(|(index, _)| built.module(index))
        .map(|module| module.name.as_str())
        .collect();

    JsEngine::new(project.fs.clone(), options)?.run_tests(&user_modules)?;
    Ok(())
}

/// A signature of `smain` that a run accepts.
pub struct SmainSignature {
    arguments: fn() -> Vec<Arc<Type>>,
    /// The name the launcher knows the signature by. `try_main` hands the
    /// arguments for it.
    kind: &'static str,
    /// How the error for an invalid `smain` shows the signature.
    pub text: &'static str,
}

pub const SMAIN_SIGNATURES: &[SmainSignature] = &[
    SmainSignature {
        arguments: Vec::new,
        kind: "Smain",
        text: "fn() -> a",
    },
    SmainSignature {
        arguments: || vec![type_::string()],
        kind: "SmainStdin",
        text: "fn(String) -> a",
    },
    SmainSignature {
        arguments: || vec![type_::list(type_::string())],
        kind: "SmainStdinLines",
        text: "fn(List(String)) -> a",
    },
];

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

    let args = smain
        .arguments
        .iter()
        .map(|arg| arg.type_.clone())
        .collect::<Vec<_>>();
    let accepts = |signature: &&SmainSignature| {
        let expected = (signature.arguments)();
        args.len() == expected.len() && args.iter().zip(&expected).all(|(a, e)| a.same_as(e))
    };
    match SMAIN_SIGNATURES.iter().find(accepts) {
        Some(signature) => Ok(MainFunction::Smain {
            kind: signature.kind,
        }),
        None => Err(SgleamError::InvalidSMain {
            module: module.name.clone(),
            signature: fn_type_to_string(module, &args[..], smain.return_type.clone()).into(),
        }),
    }
}

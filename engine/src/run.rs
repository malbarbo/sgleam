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

/// `args` are what `smain` finds in its env.
pub fn run_main(
    paths: &[Utf8PathBuf],
    options: HostOptions,
    args: Vec<String>,
) -> Result<(), SgleamError> {
    // The first path is the program, and the rest are what it imports.
    let program = paths.first().expect("a path to run");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: program.clone(),
    })?;
    run_module(&project, module, program, options, args)
}

/// Runs the `main` or the `smain` of `module`, which `project` built. An
/// error names the module by `path`.
pub fn run_module(
    project: &Project,
    module: &Module,
    path: &Utf8Path,
    options: HostOptions,
    args: Vec<String>,
) -> Result<(), SgleamError> {
    let main = get_main(module, args).map_err(|err| match err {
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
    /// Returns `true` if `smain` with these argument types has this
    /// signature, `false` otherwise.
    accepts: fn(&[Arc<Type>]) -> bool,
    /// The name the launcher knows the signature by. `try_main` hands the
    /// arguments for it.
    kind: &'static str,
    /// How the error for an invalid `smain` shows the signature.
    pub text: &'static str,
}

pub const SMAIN_SIGNATURES: &[SmainSignature] = &[
    SmainSignature {
        accepts: |args| args.is_empty(),
        kind: "Smain",
        text: "fn() -> a",
    },
    SmainSignature {
        accepts: |args| matches!(args, [arg] if arg.same_as(&type_::string())),
        kind: "SmainStdin",
        text: "fn(String) -> a",
    },
    SmainSignature {
        accepts: |args| matches!(args, [arg] if arg.same_as(&type_::list(type_::string()))),
        kind: "SmainStdinLines",
        text: "fn(List(String)) -> a",
    },
    SmainSignature {
        accepts: |args| {
            matches!(args, [arg] if arg.named_type_name()
                .is_some_and(|(module, name)| module == "sgleam/env" && name == "Env"))
        },
        kind: "SmainEnv",
        text: "fn(Env) -> a",
    },
];

fn get_main(module: &Module, args: Vec<String>) -> Result<MainFunction, SgleamError> {
    match get_smain(module, args) {
        r @ Ok(_) | r @ Err(SgleamError::InvalidSMain { .. }) => r,
        _ => Ok(module
            .ast
            .type_info
            .get_main_function(Target::JavaScript)
            .map(|_| MainFunction::Main)?),
    }
}

fn get_smain(module: &Module, args: Vec<String>) -> Result<MainFunction, SgleamError> {
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

    let types = smain
        .arguments
        .iter()
        .map(|arg| arg.type_.clone())
        .collect::<Vec<_>>();
    match SMAIN_SIGNATURES
        .iter()
        .find(|signature| (signature.accepts)(&types))
    {
        Some(signature) => Ok(MainFunction::Smain {
            kind: signature.kind,
            args,
        }),
        None => Err(SgleamError::InvalidSMain {
            module: module.name.clone(),
            signature: fn_type_to_string(module, &types[..], smain.return_type.clone()).into(),
        }),
    }
}

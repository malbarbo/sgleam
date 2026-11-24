use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};

use gleam_core::{
    build::{Module, Origin, Target},
    type_::{self, Type},
};

use crate::{
    coverage,
    engine::{Engine, MainFunction, SMAIN},
    error::SgleamError,
    gleam::{Built, Project, copy_files_and_build, fn_type_to_string, get_function},
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
    let main = get_main(module).map_err(|err| match err {
        SgleamError::Gleam(gleam_core::Error::ModuleDoesNotHaveMainFunction { module, .. }) => {
            SgleamError::NoMainFunction {
                module,
                path: path.into(),
            }
        }
        err => err,
    })?;
    if !args.is_empty() && main != (MainFunction::Smain { kind: SMAIN_ENV }) {
        return Err(SgleamError::ArgsWithoutEnv {
            path: path.into(),
            has_smain: main != MainFunction::Main,
        });
    }
    let engine = JsEngine::new(project.fs.clone(), options)?;
    engine.set_args(args)?;
    engine.run_main(&module.name, main)?;

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
    let user_modules = user_modules(user_files, paths, &built);
    JsEngine::new(project.fs.clone(), options)?.run_tests(&user_modules)?;
    Ok(())
}

/// Runs the examples as `run_test` does, with the coverage marks in every
/// file of `paths`. The files come back with the regions that ran, also when
/// an example fails or a panic stops the run. They are empty when the run
/// does not start or Ctrl-C stops it.
pub fn run_test_coverage(
    user_files: &[Utf8PathBuf],
    paths: &[Utf8PathBuf],
    options: HostOptions,
) -> (Vec<coverage::File>, Result<(), SgleamError>) {
    let mut files = vec![];
    let result = run_coverage(user_files, paths, options, &mut files);
    if matches!(result, Err(SgleamError::Interrupted)) {
        files.clear();
    }
    (files, result)
}

fn run_coverage(
    user_files: &[Utf8PathBuf],
    paths: &[Utf8PathBuf],
    options: HostOptions,
    files: &mut Vec<coverage::File>,
) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let user_modules = user_modules(user_files, paths, &built);
    let compiled = coverage::compile(&mut project, paths, &built)?;
    let engine = JsEngine::new(project.fs.clone(), options)?;
    let result = engine.run_tests(&user_modules);
    if matches!(result, Err(SgleamError::Interrupted)) {
        return result;
    }
    // The context outlives a failed example and a panic, so the marks are
    // still there to read.
    let hits = engine.coverage()?;
    *files = compiled;
    let regions = files.iter_mut().flat_map(|file| file.regions.iter_mut());
    for (n, region) in regions.enumerate() {
        region.ran = hits.binary_search(&n).is_ok();
    }
    result
}

/// The names of the modules that `user_files` became, in the order of
/// `paths`.
fn user_modules<'a>(
    user_files: &[Utf8PathBuf],
    paths: &[Utf8PathBuf],
    built: &'a Built,
) -> Vec<&'a str> {
    paths
        .iter()
        .enumerate()
        .filter(|(_, path)| user_files.contains(path))
        .filter_map(|(index, _)| built.module(index))
        .map(|module| module.name.as_str())
        .collect()
}

/// The kind of `smain(env: Env)`, the only entry point that receives the
/// command-line arguments.
const SMAIN_ENV: &str = "SmainEnv";

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
        kind: SMAIN_ENV,
        text: "fn(Env) -> a",
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
        }),
        None => Err(SgleamError::InvalidSMain {
            module: module.name.clone(),
            signature: fn_type_to_string(module, &types[..], smain.return_type.clone()).into(),
        }),
    }
}

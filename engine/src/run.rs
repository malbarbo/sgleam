use std::{fmt::Write, sync::Arc};

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
    swriteln,
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
    let main = entry_point(module, path, Target::JavaScript, &args)?;
    let engine = JsEngine::new(project.fs.clone(), options)?;
    engine.set_args(args)?;
    engine.run_main(&module.name, main)?;

    Ok(())
}

/// The WebAssembly program of the first of `paths`, which imports the rest.
/// The program reads its arguments when it runs, and `args` only tell if it
/// takes them.
pub fn build_webassembly(paths: &[Utf8PathBuf], args: &[String]) -> Result<Vec<u8>, SgleamError> {
    let program = paths.first().expect("a path to build");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: program.clone(),
    })?;
    let name = match entry_point(module, program, Target::WebAssembly, args)? {
        MainFunction::Smain { kind } => {
            let launcher = launcher(module, smain_argument(kind));
            project.write_source(&format!("{LAUNCHER}.gleam"), &launcher);
            LAUNCHER
        }
        _ => &module.name,
    };
    project.compile_webassembly()?;
    Ok(project.read_webassembly(name)?)
}

/// The WebAssembly program that runs the `_examples` functions of the first
/// of `paths`, as `run_test` runs them on JavaScript.
pub fn build_webassembly_tests(paths: &[Utf8PathBuf]) -> Result<Vec<u8>, SgleamError> {
    let program = paths.first().expect("a path to build");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: program.clone(),
    })?;
    project.write_source(&format!("{LAUNCHER}.gleam"), &tests_launcher(module));
    project.compile_webassembly()?;
    Ok(project.read_webassembly(LAUNCHER)?)
}

/// The source of `LAUNCHER` that runs each public `_examples` function of
/// `module` without arguments with `check.run_tests`, in the order of their
/// names, and exits with 1 when one does not pass.
fn tests_launcher(module: &Module) -> String {
    let mut names: Vec<_> = module
        .ast
        .definitions
        .functions
        .iter()
        .filter(|f| f.publicity.is_public() && f.arguments.is_empty())
        .filter_map(|f| f.name.as_ref().map(|(_, name)| name.as_str()))
        .filter(|name| name.ends_with("_examples"))
        .collect();
    names.sort_unstable();
    let mut tests = String::new();
    for name in names {
        // A test gives back what its last check gave, which `run_tests` does
        // not take.
        swriteln!(
            &mut tests,
            "    #(\"{module}.gleam\", \"{name}\", fn() {{\n      let _ = program.{name}()\n      Nil\n    }}),",
            module = module.name
        );
    }
    format!(
        "import sgleam/check\nimport sgleam/wasi\nimport {} as program\n\n\
         pub fn main() {{\n  case check.run_tests([\n{tests}  ]) {{\n    \
         True -> Nil\n    False -> wasi.exit(1)\n  }}\n}}\n",
        module.name
    )
}

/// The module whose `main` calls the `smain` of a program on the WebAssembly
/// target, which has no launcher of its own.
const LAUNCHER: &str = "sgleam/launcher";

/// The argument, as a Gleam expression, that `try_main` passes to an `smain`
/// of `kind`. The program reads stdin and the env from WASI when it starts,
/// as `try_main` reads them.
fn smain_argument(kind: &str) -> &'static str {
    match kind {
        SMAIN_STDIN => "string.join(wasi.stdin_lines(), \"\\n\")",
        SMAIN_STDIN_LINES => "wasi.stdin_lines()",
        SMAIN_ENV => "env.from_host()",
        _ => "",
    }
}

/// The source of `LAUNCHER` for `module`, which passes `argument` to
/// `smain`. It prints the result of `smain` as `try_main` does, with the
/// printer that the type of the result picks.
fn launcher(module: &Module, argument: &str) -> String {
    let smain = get_function(module, SMAIN).expect("a module with smain");
    let call = format!("program.smain({argument})");
    let show = if smain.return_type.same_as(&type_::string()) {
        format!("io.println({call})")
    } else if smain.return_type.same_as(&type_::nil()) {
        call
    } else {
        format!("io.println(string.inspect({call}))")
    };
    // The alias keeps a module of the user's named `io` or `env` apart from
    // the modules of the launcher.
    format!(
        "import gleam/io\nimport gleam/string\nimport sgleam/env\nimport sgleam/wasi\n\
         import {} as program\n\npub fn main() {{\n  {show}\n}}\n",
        module.name
    )
}

/// The `main` or the `smain` of `module` on `target`, which receives `args`.
/// An error names the module by `path`.
fn entry_point(
    module: &Module,
    path: &Utf8Path,
    target: Target,
    args: &[String],
) -> Result<MainFunction, SgleamError> {
    let main = get_main(module, target).map_err(|err| match err {
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
    Ok(main)
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

/// The kind of `smain(env: Env)`, the only entry point that receives the
/// command-line arguments.
const SMAIN_ENV: &str = "SmainEnv";

const SMAIN_STDIN: &str = "SmainStdin";

const SMAIN_STDIN_LINES: &str = "SmainStdinLines";

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
        kind: SMAIN_STDIN,
        text: "fn(String) -> a",
    },
    SmainSignature {
        accepts: |args| matches!(args, [arg] if arg.same_as(&type_::list(type_::string()))),
        kind: SMAIN_STDIN_LINES,
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

fn get_main(module: &Module, target: Target) -> Result<MainFunction, SgleamError> {
    match get_smain(module, target) {
        r @ Ok(_) | r @ Err(SgleamError::InvalidSMain { .. }) => r,
        smain => match module.ast.type_info.get_main_function(target) {
            Ok(_) => Ok(MainFunction::Main),
            // The module has an entry point, the `smain` that the target
            // does not support.
            Err(gleam_core::Error::ModuleDoesNotHaveMainFunction { .. })
                if matches!(
                    smain,
                    Err(SgleamError::Gleam(
                        gleam_core::Error::MainFunctionDoesNotSupportTarget { .. }
                    ))
                ) =>
            {
                smain
            }
            Err(err) => Err(err.into()),
        },
    }
}

fn get_smain(module: &Module, target: Target) -> Result<MainFunction, SgleamError> {
    let smain = get_function(module, SMAIN).ok_or_else(|| {
        gleam_core::Error::ModuleDoesNotHaveMainFunction {
            module: module.name.clone(),
            origin: Origin::Src,
        }
    })?;

    if !smain.implementations.supports(target) {
        return Err(gleam_core::Error::MainFunctionDoesNotSupportTarget {
            module: module.name.clone(),
            target,
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

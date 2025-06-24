use camino::{Utf8Path, Utf8PathBuf};

use gleam_core::{
    ast::{TypedDefinition, TypedFunction},
    build::{Module, Origin, Target},
};

use crate::{
    engine::{Engine, MainFunction},
    error::{show_error, SgleamError},
    gleam::{compile, fn_type_to_string, get_module, type_to_string, Project},
    repl::{welcome_message, Repl, ReplOutput},
    repl_reader::ReplReader,
};

use crate::quickjs::QuickJsEngine as JsEngine;

const SGLEAM_SMAIN: &str = "smain";

pub fn run_interative(paths: &[Utf8PathBuf], quiet: bool) -> Result<(), SgleamError> {
    if !quiet {
        print!("{}", welcome_message());
    }

    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, paths)?;
    let module = paths.first().and_then(|input| {
        let name = input.with_extension("");
        let name = name.as_str().replace('\\', "/");
        get_module(&modules, &name)
    });

    let mut repl = Repl::<JsEngine>::new(project, module)?;
    for input in ReplReader::new()? {
        match repl.run(&input) {
            Err(err) => show_error(&err),
            Ok(ReplOutput::Quit) => break,
            _ => continue,
        }
    }

    Ok(())
}

pub fn run_main(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, paths)?;
    let name = paths[0].with_extension("");
    let name = name.as_str().replace('\\', "/");

    if let Some(module) = get_module(&modules, &name) {
        let main = get_main(module)?;
        JsEngine::new(project.fs.clone()).run_main(&module.name, main, main != MainFunction::Main);
    } else {
        // The compiler ignored the file because of the name and printed a warning.
    }

    Ok(())
}

pub fn run_check(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    Ok(copy_files_and_build(&mut project, paths).map(|_| ())?)
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

    JsEngine::new(project.fs.clone()).run_tests(&modules);
    Ok(())
}

pub fn get_function<'a>(module: &'a Module, name: &str) -> Option<&'a TypedFunction> {
    module.ast.definitions.iter().find_map(|def| match def {
        TypedDefinition::Function(f) if f.name.as_ref().map(|s| s.1.as_str()) == Some(name) => {
            Some(f)
        }
        _ => None,
    })
}

pub fn get_main(module: &Module) -> Result<MainFunction, SgleamError> {
    let main = match get_smain(module) {
        r @ Ok(_) | r @ Err(SgleamError::InvalidSMain { .. }) => return r,
        _ => module
            .ast
            .type_info
            .get_main_function(Target::JavaScript)
            .map(|_| MainFunction::Main)?,
    };
    Ok(main)
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

    match &smain.arguments[..] {
        [] => Ok(MainFunction::Smain),
        // TODO: make the signatures generic, also in show_error
        [arg] if type_to_string(module, &arg.type_) == "String" => Ok(MainFunction::SmainStdin),
        [arg] if type_to_string(module, &arg.type_) == "List(String)" => {
            Ok(MainFunction::SmainStdinLines)
        }
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

fn copy_files_and_build(
    project: &mut Project,
    paths: &[Utf8PathBuf],
) -> Result<Vec<Module>, gleam_core::Error> {
    for path in paths.iter().filter(|p| validade_path(p)) {
        project.copy_file_to_source(path)?;
    }
    let mut modules = compile(project, false)?;
    modules
        .retain(|module| !module.name.starts_with("gleam/") && !module.name.starts_with("sgleam/"));
    Ok(modules)
}

fn validade_path(path: &Utf8Path) -> bool {
    let steam = path.file_stem().unwrap_or("");
    if path.extension() != Some("gleam") || steam.is_empty() {
        eprintln!("Ignoring `{path}`: is not a valid gleam file.");
        return false;
    }

    if steam == "gleam" || steam == "sgleam" {
        eprintln!("Ignoring `{path}`: `{steam}` is a reserved module name.");
        return false;
    }

    true
}

use camino::{Utf8Path, Utf8PathBuf};
use ecow::EcoString;

use gleam_core::{
    ast::TypedFunction,
    build::{Module, Origin, Target},
    type_,
};

use crate::{
    engine::{Engine, MainFunction},
    error::SgleamError,
    gleam::{Project, fn_type_to_string, get_module, is_module_path},
};

use crate::quickjs::QuickJsEngine as JsEngine;

const SGLEAM_SMAIN: &str = "smain";

pub fn run_main(paths: &[Utf8PathBuf]) -> Result<(), SgleamError> {
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, paths)?;
    let module = built.module(0).ok_or_else(|| SgleamError::NoModuleToRun {
        path: paths[0].clone(),
    })?;

    let main = get_main(module)?;
    let show_output = main != MainFunction::Main;
    JsEngine::new(project.fs.clone())?.run_main(&module.name, main, show_output)?;

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

/// What a build knows and no one can work out from the paths alone: the module
/// each path became, in the order the caller gave, and `None` for a path the
/// build never copied.
pub struct Built {
    pub modules: Vec<Module>,
    pub names: Vec<Option<EcoString>>,
}

impl Built {
    /// The module a given path became, under the name the copy gave it — and
    /// not a name derived from the path a second time.
    pub fn module(&self, index: usize) -> Option<&Module> {
        let name = self.names.get(index)?.as_ref()?;
        get_module(&self.modules, name)
    }
}

pub fn copy_files_and_build(
    project: &mut Project,
    paths: &[Utf8PathBuf],
) -> Result<Built, gleam_core::Error> {
    let mut names = Vec::with_capacity(paths.len());
    for path in paths {
        names.push(if validate_path(path) {
            Some(project.copy_file_to_source(path)?)
        } else {
            None
        });
    }
    let mut modules = project.compile(false)?;
    modules
        .retain(|module| !module.name.starts_with("gleam/") && !module.name.starts_with("sgleam/"));
    Ok(Built { modules, names })
}

fn validate_path(path: &Utf8Path) -> bool {
    // The path is the module name, so it cannot leave the current directory.
    if !is_module_path(path.as_str()) {
        eprintln!("Ignoring `{path}`: is not a path within the current directory.");
        return false;
    }

    let stem = path.file_stem().unwrap_or("");
    if path.extension() != Some("gleam") || stem.is_empty() {
        eprintln!("Ignoring `{path}`: is not a valid gleam file.");
        return false;
    }

    if stem == "gleam" || stem == "sgleam" {
        eprintln!("Ignoring `{path}`: `{stem}` is a reserved module name.");
        return false;
    }

    if let Some(dir @ ("gleam" | "sgleam")) = path.iter().next() {
        eprintln!("Ignoring `{path}`: `{dir}` is a reserved directory.");
        return false;
    }

    true
}

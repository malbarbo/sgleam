use camino::Utf8PathBuf;
use gleam_core::ast::Statement;

use crate::{
    error::SgleamError,
    gleam::{Project, get_module},
    run::copy_files_and_build,
    substitution::{SubstitutionModule, SubstitutionStep},
};

pub fn build_stepper(path: Utf8PathBuf) -> Result<Vec<SubstitutionStep>, SgleamError> {
    let paths = crate::gleam::find_imports(vec![path.clone()])?;
    let mut project = Project::default();
    let modules = copy_files_and_build(&mut project, &paths)?;

    let name = path.with_extension("");
    let name = name.as_str().replace('\\', "/");
    let module =
        get_module(&modules, &name).ok_or_else(|| SgleamError::Other("Module not found".into()))?;

    let main = module
        .ast
        .definitions
        .functions
        .iter()
        .find(|f| f.name.as_ref().map(|n| n.1.as_str()) == Some("main"))
        .ok_or_else(|| SgleamError::Other("Function main not found".into()))?;

    let expr = match main.body.as_slice() {
        [Statement::Expression(expr)] => expr.clone(),
        _ => {
            return Err(SgleamError::Other(
                "main function has multiple statements; use a single expression like `1 + 2`"
                    .into(),
            ));
        }
    };

    let mut validator = crate::substitution::validate::Validator::new(
        module.code.clone(),
        module.input_path.clone(),
    );
    validator.validate_expr(&expr)?;

    let untyped_expr = crate::substitution::convert::typed_to_untyped(&expr)?;

    let mut substitution_module = SubstitutionModule::default();
    for module in modules {
        substitution_module.merge(SubstitutionModule::from_module(&module));
    }

    let trace = substitution_module.evaluate(&untyped_expr, 1000)?;

    Ok(trace.into_steps())
}

use std::collections::HashMap;

use ecow::EcoString;
use gleam_core::ast::{SrcSpan, UntypedExpr};
use gleam_core::build::Module;
use thiserror::Error;

pub mod convert;
pub mod reduce;
pub mod render;
pub mod runtime_vars;
pub mod validate;

use render::render_expr;
use validate::Validator;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SubstitutionStep {
    pub expr: UntypedExpr,
    pub formatted: String,
    pub note: Option<String>,
    pub context: Option<String>,
}

impl SubstitutionStep {
    pub fn new(expr: UntypedExpr, note: Option<String>) -> Self {
        let formatted = render_expr(&expr).expect("Formatting failed in SubstitutionStep::new");
        Self {
            expr,
            formatted,
            note,
            context: None,
        }
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_context_opt(mut self, context: Option<String>) -> Self {
        self.context = context;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubstitutionTrace {
    steps: Vec<SubstitutionStep>,
}

impl SubstitutionTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_step(&mut self, step: SubstitutionStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[SubstitutionStep] {
        &self.steps
    }

    pub fn into_steps(self) -> Vec<SubstitutionStep> {
        self.steps
    }
}

#[derive(Debug, Clone)]
pub struct SubstitutionFunction {
    pub name: EcoString,
    pub arguments: Vec<EcoString>,
    pub body: UntypedExpr,
}

#[derive(Debug, Clone, Default)]
pub struct SubstitutionModule {
    pub functions: HashMap<EcoString, SubstitutionFunction>,
    pub unsupported_functions: HashMap<EcoString, SubstitutionError>,
}

impl SubstitutionModule {
    pub fn from_module(module: &Module) -> Self {
        let mut functions = HashMap::new();
        let mut unsupported_functions = HashMap::new();
        for function in &module.ast.definitions.functions {
            let mut validator = Validator::new(module.code.clone(), module.input_path.clone());
            let Some((_, name)) = function.name.as_ref() else {
                continue;
            };
            let arguments: Vec<EcoString> = function
                .arguments
                .iter()
                .filter_map(|arg| arg.get_variable_name().cloned())
                .collect();

            let mut validation_result = Ok(());
            for stmt in &function.body {
                if let Err(err) = validator.validate_statement(stmt) {
                    validation_result = Err(err);
                    break;
                }
            }

            match validation_result.and_then(|_| convert::typed_body_to_untyped(&function.body)) {
                Ok(body) => {
                    functions.insert(
                        name.clone(),
                        SubstitutionFunction {
                            name: name.clone(),
                            arguments,
                            body,
                        },
                    );
                }
                Err(err) => {
                    unsupported_functions.insert(name.clone(), err);
                }
            }
        }
        Self {
            functions,
            unsupported_functions,
        }
    }

    pub fn merge(&mut self, other: SubstitutionModule) {
        for (name, function) in other.functions {
            self.unsupported_functions.remove(&name);
            self.functions.insert(name, function);
        }
        for (name, err) in other.unsupported_functions {
            if !self.functions.contains_key(&name) {
                self.unsupported_functions.insert(name, err);
            }
        }
    }

    pub fn find_function(&self, name: &str) -> Option<&SubstitutionFunction> {
        self.functions.get(name)
    }

    pub fn unsupported_error(&self, name: &str) -> Option<&SubstitutionError> {
        self.unsupported_functions.get(name)
    }

    pub fn evaluate(
        &self,
        expr: &UntypedExpr,
        max_steps: usize,
    ) -> Result<SubstitutionTrace, SubstitutionError> {
        let mut trace = SubstitutionTrace::new();
        let mut current = expr.clone();
        trace.push_step(SubstitutionStep::new(current.clone(), None));

        if reduce::is_value(expr) {
            return Ok(trace);
        }

        for _ in 0..max_steps {
            match self.reduce_once(&current) {
                Ok(Some(step)) => {
                    current = step.expr.clone();
                    trace.push_step(step);
                }
                Ok(None) => return Ok(trace),
                Err(err) => return Err(err),
            }
        }

        Err(SubstitutionError::StepLimitExceeded(max_steps))
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

use camino::Utf8PathBuf;

#[derive(Debug, Error, Clone)]
pub enum SubstitutionError {
    #[error("The stepper does not support {kind} yet")]
    UnsupportedFeature {
        kind: String,
        location: SrcSpan,
        src: EcoString,
        path: Utf8PathBuf,
    },

    #[error("Step limit exceeded after {0} steps")]
    StepLimitExceeded(usize),

    #[error("Internal formatting error")]
    FormattingError,

    #[error(transparent)]
    Format(#[from] gleam_core::Error),
}

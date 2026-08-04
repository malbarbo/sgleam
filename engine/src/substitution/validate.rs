use camino::Utf8PathBuf;
use ecow::EcoString;
use gleam_core::ast::{
    TypedExpr, TypedStatement,
    visit::{Visit, visit_typed_expr, visit_typed_statement},
};

use super::SubstitutionError;

pub struct Validator {
    pub src: EcoString,
    pub path: Utf8PathBuf,
    pub error: Option<SubstitutionError>,
}

impl Validator {
    pub fn new(src: EcoString, path: Utf8PathBuf) -> Self {
        Self {
            src,
            path,
            error: None,
        }
    }

    pub fn validate_expr(&mut self, expr: &TypedExpr) -> Result<(), SubstitutionError> {
        self.visit_typed_expr(expr);
        if let Some(err) = self.error.take() {
            return Err(err);
        }
        Ok(())
    }

    pub fn validate_statement(&mut self, stmt: &TypedStatement) -> Result<(), SubstitutionError> {
        self.visit_typed_statement(stmt);
        if let Some(err) = self.error.take() {
            return Err(err);
        }
        Ok(())
    }

    fn unsupported(&mut self, kind: &str, location: gleam_core::ast::SrcSpan) {
        self.error = Some(SubstitutionError::UnsupportedFeature {
            kind: kind.into(),
            location,
            src: self.src.clone(),
            path: self.path.clone(),
        });
    }
}

impl<'a> Visit<'a> for Validator {
    fn visit_typed_expr(&mut self, expr: &'a TypedExpr) {
        if self.error.is_some() {
            return;
        }

        match expr {
            TypedExpr::BitArray { location, .. } => {
                self.unsupported("BitArrays", *location);
            }
            TypedExpr::RecordUpdate { location, .. } => {
                self.unsupported("record updates", *location);
            }
            TypedExpr::Fn { location, .. } => {
                self.unsupported("anonymous functions", *location);
            }
            TypedExpr::RecordAccess { location, .. } => {
                self.unsupported("record access", *location);
            }
            TypedExpr::TupleIndex { location, .. } => {
                self.unsupported("tuple indexing", *location);
            }
            TypedExpr::Echo { location, .. } => {
                self.unsupported("echo statements", *location);
            }
            TypedExpr::Pipeline { .. } => {
                self.unsupported("pipelines", expr.location());
            }
            TypedExpr::Todo { location, .. } => {
                self.unsupported("todo", *location);
            }
            TypedExpr::Panic { location, .. } => {
                self.unsupported("panic", *location);
            }
            TypedExpr::ModuleSelect { location, .. } => {
                self.unsupported("module access", *location);
            }
            TypedExpr::Case { clauses, .. } => {
                for clause in clauses {
                    if let Some(guard) = &clause.guard {
                        let loc = guard.location();
                        let pattern_end = clause
                            .pattern
                            .last()
                            .map(|p| p.location().end)
                            .unwrap_or(clause.location.start);

                        let snippet = &self.src[pattern_end as usize..loc.end as usize];
                        let if_offset = snippet.find("if").unwrap_or(0);

                        let location = gleam_core::ast::SrcSpan {
                            start: pattern_end + if_offset as u32,
                            end: loc.end,
                        };
                        self.unsupported("case guards", location);
                        return;
                    }
                }
                visit_typed_expr(self, expr);
            }
            _ => visit_typed_expr(self, expr),
        }
    }

    fn visit_typed_statement(&mut self, stmt: &'a TypedStatement) {
        if self.error.is_some() {
            return;
        }

        match stmt {
            TypedStatement::Use(u) => {
                self.unsupported("use statements", u.location);
            }
            TypedStatement::Assignment(assignment) => {
                if matches!(assignment.kind, gleam_core::ast::AssignmentKind::Generated) {
                    self.unsupported("generated assignments", assignment.location);
                    return;
                }
                visit_typed_statement(self, stmt);
            }
            _ => visit_typed_statement(self, stmt),
        }
    }
}

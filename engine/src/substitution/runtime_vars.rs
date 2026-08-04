//! Scope-aware visitor for finding variable references in a typed expression.
//! Used by `:stepper` to detect references to REPL runtime variables that the
//! substitution evaluator cannot resolve.

use std::collections::HashSet;

use ecow::EcoString;
use gleam_core::ast::{
    AssignName, Pattern, TypedExpr, TypedPattern, TypedStatement,
    visit::{Visit, visit_typed_expr},
};

/// Returns the name of the first variable referenced by `expr` that is in
/// `target_names`, respecting local scoping (`let` bindings, function
/// arguments, and case clause patterns shadow names from `target_names`).
pub fn find_runtime_var_ref(expr: &TypedExpr, target_names: &HashSet<String>) -> Option<String> {
    let mut visitor = ReferencedVarVisitor::new(target_names);
    visitor.visit_typed_expr(expr);
    visitor.found.map(|name| name.to_string())
}

struct ReferencedVarVisitor {
    targets: HashSet<EcoString>,
    found: Option<EcoString>,
}

impl ReferencedVarVisitor {
    fn new(target_names: &HashSet<String>) -> Self {
        let targets = target_names
            .iter()
            .map(|name| EcoString::from(name.as_str()))
            .collect();
        Self {
            targets,
            found: None,
        }
    }

    fn with_local_scope(&mut self, f: impl FnOnce(&mut Self)) {
        let outer = self.targets.clone();
        f(self);
        self.targets = outer;
    }

    fn visit_statements(&mut self, statements: &[TypedStatement]) {
        for statement in statements {
            self.visit_typed_statement(statement);
            if self.found.is_some() {
                return;
            }
        }
    }

    fn remove_pattern_names(&mut self, pattern: &TypedPattern) {
        match pattern {
            Pattern::Variable { name, .. } => {
                self.targets.remove(name);
            }
            Pattern::Assign { name, pattern, .. } => {
                self.targets.remove(name);
                self.remove_pattern_names(pattern);
            }
            Pattern::List { elements, tail, .. } => {
                for element in elements {
                    self.remove_pattern_names(element);
                }
                if let Some(tail) = tail {
                    self.remove_pattern_names(&tail.pattern);
                }
            }
            Pattern::Tuple { elements, .. } => {
                for element in elements {
                    self.remove_pattern_names(element);
                }
            }
            Pattern::Constructor { arguments, .. } => {
                for argument in arguments {
                    self.remove_pattern_names(&argument.value);
                }
            }
            Pattern::BitArray { segments, .. } => {
                for segment in segments {
                    self.remove_pattern_names(&segment.value);
                }
            }
            Pattern::StringPrefix {
                left_side_assignment,
                right_side_assignment,
                ..
            } => {
                if let Some((name, _)) = left_side_assignment {
                    self.targets.remove(name);
                }
                if let AssignName::Variable(name) = right_side_assignment {
                    self.targets.remove(name);
                }
            }
            Pattern::Int { .. }
            | Pattern::Float { .. }
            | Pattern::String { .. }
            | Pattern::Discard { .. }
            | Pattern::BitArraySize(_)
            | Pattern::Invalid { .. } => {}
        }
    }
}

impl<'ast> Visit<'ast> for ReferencedVarVisitor {
    fn visit_typed_expr(&mut self, expr: &'ast TypedExpr) {
        if self.found.is_some() {
            return;
        }

        match expr {
            TypedExpr::Var { name, .. } => {
                if self.targets.contains(name) {
                    self.found = Some(name.clone());
                }
            }
            TypedExpr::Block { statements, .. } => {
                self.with_local_scope(|this| this.visit_statements(statements));
            }
            TypedExpr::Fn {
                arguments, body, ..
            } => {
                self.with_local_scope(|this| {
                    for argument in arguments {
                        if let Some(name) = argument.get_variable_name() {
                            this.targets.remove(name);
                        }
                    }
                    this.visit_statements(body.as_slice());
                });
            }
            TypedExpr::Case {
                subjects, clauses, ..
            } => {
                for subject in subjects {
                    self.visit_typed_expr(subject);
                    if self.found.is_some() {
                        return;
                    }
                }

                for clause in clauses {
                    self.with_local_scope(|this| {
                        for pattern in &clause.pattern {
                            this.remove_pattern_names(pattern);
                        }
                        for alternative in &clause.alternative_patterns {
                            for pattern in alternative {
                                this.remove_pattern_names(pattern);
                            }
                        }
                        this.visit_typed_expr(&clause.then);
                    });
                    if self.found.is_some() {
                        return;
                    }
                }
            }
            _ => visit_typed_expr(self, expr),
        }
    }

    fn visit_typed_statement(&mut self, statement: &'ast TypedStatement) {
        if self.found.is_some() {
            return;
        }

        match statement {
            TypedStatement::Expression(expr) => self.visit_typed_expr(expr),
            TypedStatement::Assignment(assignment) => {
                self.visit_typed_expr(&assignment.value);
                self.remove_pattern_names(&assignment.pattern);
            }
            TypedStatement::Use(use_) => self.visit_typed_expr(&use_.call),
            TypedStatement::Assert(assert_) => {
                self.visit_typed_expr(&assert_.value);
                if let Some(message) = &assert_.message {
                    self.visit_typed_expr(message);
                }
            }
        }
    }
}

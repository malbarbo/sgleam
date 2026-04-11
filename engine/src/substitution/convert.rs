use gleam_core::ast::{
    AssignmentKind, CallArg, Clause, Pattern, SrcSpan, Statement, TailPattern, TypedExpr,
    UntypedExpr, UntypedStatement,
};
use std::sync::Arc;
use vec1::Vec1;

use super::SubstitutionError;

const S: SrcSpan = SrcSpan { start: 0, end: 0 };

/// Converts a function body (Vec1<TypedStatement>) to a single UntypedExpr.
pub fn typed_body_to_untyped(
    body: &[gleam_core::ast::TypedStatement],
) -> Result<UntypedExpr, SubstitutionError> {
    if let [Statement::Expression(expr)] = body {
        return typed_to_untyped(expr);
    }
    let stmts: Vec<UntypedStatement> = body
        .iter()
        .map(convert_statement)
        .collect::<Result<Vec<_>, _>>()?;

    let stmts = Vec1::try_from_vec(stmts).unwrap_or_else(|_| {
        Vec1::try_from(vec![Statement::Expression(UntypedExpr::Var {
            location: S,
            name: "Nil".into(),
        })])
        .unwrap()
    });

    Ok(UntypedExpr::Block {
        location: S,
        statements: stmts,
    })
}

pub fn typed_to_untyped(expr: &TypedExpr) -> Result<UntypedExpr, SubstitutionError> {
    match expr {
        TypedExpr::Int {
            value, int_value, ..
        } => Ok(UntypedExpr::Int {
            location: S,
            value: value.clone(),
            int_value: int_value.clone(),
        }),

        TypedExpr::Float {
            value, float_value, ..
        } => Ok(UntypedExpr::Float {
            location: S,
            value: value.clone(),
            float_value: *float_value,
        }),

        TypedExpr::String { value, .. } => Ok(UntypedExpr::String {
            location: S,
            value: value.clone(),
        }),

        TypedExpr::Var { name, .. } => Ok(UntypedExpr::Var {
            location: S,
            name: name.clone(),
        }),

        TypedExpr::BinOp {
            name,
            name_location,
            left,
            right,
            ..
        } => Ok(UntypedExpr::BinOp {
            location: S,
            name: *name,
            name_location: *name_location,
            left: Box::new(typed_to_untyped(left)?),
            right: Box::new(typed_to_untyped(right)?),
        }),

        TypedExpr::Call { fun, arguments, .. } => Ok(UntypedExpr::Call {
            location: S,
            fun: Box::new(typed_to_untyped(fun)?),
            arguments: arguments
                .iter()
                .map(convert_call_arg)
                .collect::<Result<Vec<_>, _>>()?,
        }),

        TypedExpr::List { elements, tail, .. } => Ok(UntypedExpr::List {
            location: S,
            elements: elements
                .iter()
                .map(typed_to_untyped)
                .collect::<Result<Vec<_>, _>>()?,
            tail: tail
                .as_ref()
                .map(|t| typed_to_untyped(t).map(Box::new))
                .transpose()?,
        }),

        TypedExpr::Tuple { elements, .. } => Ok(UntypedExpr::Tuple {
            location: S,
            elements: elements
                .iter()
                .map(typed_to_untyped)
                .collect::<Result<Vec<_>, _>>()?,
        }),

        TypedExpr::NegateInt { value, .. } => Ok(UntypedExpr::NegateInt {
            location: S,
            value: Box::new(typed_to_untyped(value)?),
        }),

        TypedExpr::NegateBool { value, .. } => Ok(UntypedExpr::NegateBool {
            location: S,
            value: Box::new(typed_to_untyped(value)?),
        }),

        TypedExpr::Case {
            subjects, clauses, ..
        } => Ok(UntypedExpr::Case {
            location: S,
            subjects: subjects
                .iter()
                .map(typed_to_untyped)
                .collect::<Result<Vec<_>, _>>()?,
            clauses: Some(
                clauses
                    .iter()
                    .map(convert_clause)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        }),

        TypedExpr::Block { statements, .. } => {
            let stmts: Vec<UntypedStatement> = statements
                .iter()
                .map(convert_statement)
                .collect::<Result<Vec<_>, _>>()?;
            let stmts = Vec1::try_from_vec(stmts).expect("Empty block caught by validator");
            Ok(UntypedExpr::Block {
                location: S,
                statements: stmts,
            })
        }

        _ => panic!(
            "Unsupported expression should have been caught by validator: {:#?}",
            expr
        ),
    }
}

fn convert_call_arg(arg: &CallArg<TypedExpr>) -> Result<CallArg<UntypedExpr>, SubstitutionError> {
    Ok(CallArg {
        label: arg.label.clone(),
        location: S,
        value: typed_to_untyped(&arg.value)?,
        implicit: arg.implicit,
    })
}

fn convert_clause(
    clause: &Clause<TypedExpr, Arc<gleam_core::type_::Type>, ecow::EcoString>,
) -> Result<Clause<UntypedExpr, (), ()>, SubstitutionError> {
    Ok(Clause {
        location: S,
        pattern: clause
            .pattern
            .iter()
            .map(convert_pattern)
            .collect::<Vec<_>>(),
        alternative_patterns: vec![],
        guard: None,
        then: typed_to_untyped(&clause.then)?,
    })
}

fn convert_statement(
    stmt: &gleam_core::ast::TypedStatement,
) -> Result<UntypedStatement, SubstitutionError> {
    match stmt {
        Statement::Expression(expr) => Ok(Statement::Expression(typed_to_untyped(expr)?)),
        Statement::Assignment(assignment) => {
            let assignment = assignment.as_ref();
            Ok(Statement::Assignment(Box::new(
                gleam_core::ast::Assignment {
                    location: S,
                    value: typed_to_untyped(&assignment.value)?,
                    pattern: convert_pattern(&assignment.pattern),
                    kind: match &assignment.kind {
                        AssignmentKind::Let => AssignmentKind::Let,
                        AssignmentKind::Assert {
                            location,
                            assert_keyword_start,
                            message,
                        } => AssignmentKind::Assert {
                            location: *location,
                            assert_keyword_start: *assert_keyword_start,
                            message: message.as_ref().map(|m| typed_to_untyped(m).unwrap()),
                        },
                        AssignmentKind::Generated => AssignmentKind::Generated,
                    },
                    annotation: None,
                    compiled_case: assignment.compiled_case.clone(),
                },
            )))
        }
        _ => panic!("Unsupported statement should have been caught by validator"),
    }
}

fn convert_pattern(pattern: &gleam_core::ast::TypedPattern) -> Pattern<()> {
    match pattern {
        Pattern::Int {
            value, int_value, ..
        } => Pattern::Int {
            location: S,
            value: value.clone(),
            int_value: int_value.clone(),
        },
        Pattern::Float {
            value, float_value, ..
        } => Pattern::Float {
            location: S,
            value: value.clone(),
            float_value: *float_value,
        },
        Pattern::String { value, .. } => Pattern::String {
            location: S,
            value: value.clone(),
        },
        Pattern::Variable { name, origin, .. } => Pattern::Variable {
            location: S,
            name: name.clone(),
            type_: (),
            origin: origin.clone(),
        },
        Pattern::Discard { name, .. } => Pattern::Discard {
            location: S,
            name: name.clone(),
            type_: (),
        },
        Pattern::Assign { name, pattern, .. } => Pattern::Assign {
            location: S,
            name: name.clone(),
            pattern: Box::new(convert_pattern(pattern)),
        },
        Pattern::List { elements, tail, .. } => Pattern::List {
            location: S,
            elements: elements.iter().map(convert_pattern).collect(),
            tail: tail.as_ref().map(|t| {
                Box::new(TailPattern {
                    location: S,
                    pattern: convert_pattern(&t.pattern),
                })
            }),
            type_: (),
        },
        Pattern::Tuple { elements, .. } => Pattern::Tuple {
            location: S,
            elements: elements.iter().map(convert_pattern).collect(),
        },
        Pattern::Constructor {
            name,
            arguments,
            module,
            constructor,
            name_location,
            spread,
            ..
        } => Pattern::Constructor {
            location: S,
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|a| CallArg {
                    label: a.label.clone(),
                    location: S,
                    value: convert_pattern(&a.value),
                    implicit: a.implicit,
                })
                .collect(),
            module: module.clone(),
            constructor: constructor.clone(),
            name_location: *name_location,
            spread: *spread,
            type_: (),
        },
        _ => panic!("Unsupported pattern should have been caught by validator"),
    }
}

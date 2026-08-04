use std::collections::HashMap;

use ecow::EcoString;
use gleam_core::ast::{
    BinOp, CallArg, Clause, Pattern, SrcSpan, Statement, TailPattern, UntypedExpr, UntypedStatement,
};
use num_bigint::BigInt;

use super::{SubstitutionError, SubstitutionModule, SubstitutionStep};

const S: SrcSpan = SrcSpan { start: 0, end: 0 };

impl SubstitutionModule {
    pub fn reduce_once(
        &self,
        expr: &UntypedExpr,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        match expr {
            UntypedExpr::Call { fun, arguments, .. } => {
                if !is_value(fun) {
                    return self.reduce_child_value(fun);
                }
                if let Some((i, inner)) = self.reduce_arguments(arguments)? {
                    let mut reduced = expr.clone();
                    let UntypedExpr::Call { arguments, .. } = &mut reduced else {
                        unreachable!();
                    };
                    arguments[i].value = inner.expr;
                    return Ok(Some(
                        SubstitutionStep::new(reduced, inner.note).with_context_opt(inner.context),
                    ));
                }
                Ok(Some(self.reduce_call(expr)?))
            }

            UntypedExpr::NegateBool { value, .. } | UntypedExpr::NegateInt { value, .. }
                if !is_value(expr) =>
            {
                if let Some(inner) = self.reduce_child_value(value)? {
                    return Ok(Some(rebuild_negate(expr, inner)));
                }
                Ok(Some(self.reduce_primitive(expr)?))
            }

            UntypedExpr::BinOp {
                name: BinOp::And,
                left,
                right,
                ..
            } => self.reduce_short_circuit(expr, left, right, false, "short-circuit &&"),
            UntypedExpr::BinOp {
                name: BinOp::Or,
                left,
                right,
                ..
            } => self.reduce_short_circuit(expr, left, right, true, "short-circuit ||"),

            UntypedExpr::BinOp { left, right, .. } => {
                if !is_value(left) {
                    return self.reduce_in_binop_left(expr, left);
                }
                if !is_value(right) {
                    return self.reduce_in_binop_right(expr, right);
                }
                Ok(Some(self.reduce_primitive(expr)?))
            }

            UntypedExpr::Case { .. } => {
                if let Some(inner) = self.reduce_case_subjects(expr)? {
                    return Ok(Some(inner));
                }
                let (reduced, note) = reduce_case(expr);
                Ok(Some(SubstitutionStep::new(reduced, Some(note.into()))))
            }

            UntypedExpr::Block { statements, .. } => self.reduce_block(statements),

            _ if is_value(expr) => Ok(None),

            _ => panic!(
                "Unsupported expression should have been caught by validator: {:?}",
                expr
            ),
        }
    }

    fn reduce_short_circuit(
        &self,
        expr: &UntypedExpr,
        left: &UntypedExpr,
        right: &UntypedExpr,
        short_circuit_value: bool,
        note: &str,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        if !is_value(left) {
            return self.reduce_in_binop_left(expr, left);
        }
        if is_bool_lit(left, short_circuit_value) {
            return Ok(Some(SubstitutionStep::new(
                bool_expr(short_circuit_value),
                Some(note.into()),
            )));
        }
        Ok(Some(SubstitutionStep::new(
            right.clone(),
            Some("reduce short-circuit".into()),
        )))
    }

    fn reduce_child_value(
        &self,
        child: &UntypedExpr,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        self.reduce_once(child)
    }

    fn reduce_arguments(
        &self,
        arguments: &[CallArg<UntypedExpr>],
    ) -> Result<Option<(usize, SubstitutionStep)>, SubstitutionError> {
        for (i, item) in arguments.iter().enumerate() {
            if !is_value(&item.value) {
                return Ok(self.reduce_once(&item.value)?.map(|s| (i, s)));
            }
        }
        Ok(None)
    }

    fn reduce_call(&self, expr: &UntypedExpr) -> Result<SubstitutionStep, SubstitutionError> {
        let UntypedExpr::Call { fun, arguments, .. } = expr else {
            unreachable!();
        };
        let UntypedExpr::Var { name, .. } = fun.as_ref() else {
            panic!("Unsupported function reference should have been caught by validator");
        };

        let function = self.find_function(name).ok_or_else(|| {
            if let Some(err) = self.unsupported_error(name) {
                err.clone()
            } else {
                panic!("Function not found: {}", name);
            }
        })?;

        let env: HashMap<EcoString, UntypedExpr> = function
            .arguments
            .iter()
            .cloned()
            .zip(arguments.iter().map(|a| a.value.clone()))
            .collect();

        let reduced = substitute_expr(&function.body, &env);
        let mut s = SubstitutionStep::new(
            reduced,
            Some(format!("substitute call to {}", function.name)),
        );
        if let Ok(context_str) = crate::substitution::render::render_function(
            &function.name,
            &function.arguments,
            &function.body,
        ) {
            s.context = Some(context_str);
        }
        Ok(s)
    }

    fn reduce_primitive(&self, expr: &UntypedExpr) -> Result<SubstitutionStep, SubstitutionError> {
        let reduced = reduce_primitive_expr(expr);
        Ok(SubstitutionStep::new(
            reduced,
            Some("reduce primitive operator".into()),
        ))
    }

    fn reduce_in_binop_left(
        &self,
        expr: &UntypedExpr,
        left: &UntypedExpr,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        let Some(inner) = self.reduce_once(left)? else {
            return Ok(None);
        };
        let mut reduced = expr.clone();
        let UntypedExpr::BinOp { left, .. } = &mut reduced else {
            unreachable!();
        };
        **left = inner.expr;
        Ok(Some(
            SubstitutionStep::new(reduced, inner.note).with_context_opt(inner.context),
        ))
    }

    fn reduce_in_binop_right(
        &self,
        expr: &UntypedExpr,
        right: &UntypedExpr,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        let Some(inner) = self.reduce_once(right)? else {
            return Ok(None);
        };
        let mut reduced = expr.clone();
        let UntypedExpr::BinOp { right, .. } = &mut reduced else {
            unreachable!();
        };
        **right = inner.expr;
        Ok(Some(
            SubstitutionStep::new(reduced, inner.note).with_context_opt(inner.context),
        ))
    }

    fn reduce_block(
        &self,
        statements: &vec1::Vec1<UntypedStatement>,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        use gleam_core::ast::Statement;

        if statements.len() == 1
            && let Statement::Expression(expr) = statements.first()
        {
            if let Some(mut inner) = self.reduce_once(expr)? {
                if inner.note.is_none() {
                    inner.note = Some("evaluate block".into());
                }
                return Ok(Some(inner));
            } else {
                return Ok(Some(SubstitutionStep::new(
                    expr.clone(),
                    Some("unwrap block".into()),
                )));
            }
        }

        for (i, stmt) in statements.iter().enumerate() {
            match stmt {
                Statement::Assignment(assignment) if is_value(&assignment.value) => {
                    let mut env = HashMap::new();
                    collect_pattern_bindings(&assignment.pattern, &assignment.value, &mut env);
                    let mut remaining: Vec<UntypedStatement> =
                        Vec::with_capacity(statements.len() - i - 1);
                    for s in statements.iter().skip(i + 1) {
                        remaining.push(substitute_statement(s, &env));
                        if let Statement::Assignment(a) = s {
                            remove_pattern_names(&a.pattern, &mut env);
                        }
                    }
                    if remaining.is_empty() {
                        return Ok(Some(SubstitutionStep::new(
                            assignment.value.clone(),
                            Some("substitute let".into()),
                        )));
                    } else {
                        return Ok(Some(SubstitutionStep::new(
                            block_from_statements(remaining)?,
                            Some("substitute let".into()),
                        )));
                    }
                }

                Statement::Assignment(assignment) => {
                    let Some(inner) = self.reduce_once(&assignment.value)? else {
                        return Ok(None);
                    };
                    let mut new_stmts: Vec<UntypedStatement> = statements.iter().cloned().collect();
                    let Statement::Assignment(ref mut asgn) = new_stmts[i] else {
                        unreachable!();
                    };
                    asgn.value = inner.expr;
                    return Ok(Some(
                        SubstitutionStep::new(block_from_statements(new_stmts)?, inner.note)
                            .with_context_opt(inner.context),
                    ));
                }

                Statement::Expression(expr) => {
                    let is_last = i + 1 == statements.len();
                    if is_last {
                        let Some(inner) = self.reduce_once(expr)? else {
                            return Ok(None);
                        };
                        let mut new_stmts: Vec<UntypedStatement> =
                            statements.iter().cloned().collect();
                        new_stmts[i] = Statement::Expression(inner.expr);
                        return Ok(Some(
                            SubstitutionStep::new(block_from_statements(new_stmts)?, inner.note)
                                .with_context_opt(inner.context),
                        ));
                    }
                    if !is_value(expr) {
                        let Some(inner) = self.reduce_once(expr)? else {
                            continue;
                        };
                        let mut new_stmts: Vec<UntypedStatement> =
                            statements.iter().cloned().collect();
                        new_stmts[i] = Statement::Expression(inner.expr);
                        return Ok(Some(
                            SubstitutionStep::new(block_from_statements(new_stmts)?, inner.note)
                                .with_context_opt(inner.context),
                        ));
                    }
                    let remaining: Vec<UntypedStatement> =
                        statements.iter().skip(i + 1).cloned().collect();
                    return Ok(Some(SubstitutionStep::new(
                        block_from_statements(remaining)?,
                        Some("discard expression".into()),
                    )));
                }

                _ => unreachable!(),
            }
        }
        Ok(None)
    }

    fn reduce_case_subjects(
        &self,
        expr: &UntypedExpr,
    ) -> Result<Option<SubstitutionStep>, SubstitutionError> {
        let UntypedExpr::Case { subjects, .. } = expr else {
            unreachable!();
        };
        for (i, subject) in subjects.iter().enumerate() {
            if !is_value(subject) {
                let Some(inner) = self.reduce_once(subject)? else {
                    continue;
                };
                let mut reduced = expr.clone();
                let UntypedExpr::Case { subjects, .. } = &mut reduced else {
                    unreachable!();
                };
                subjects[i] = inner.expr;
                return Ok(Some(
                    SubstitutionStep::new(reduced, inner.note).with_context_opt(inner.context),
                ));
            }
        }
        Ok(None)
    }
}

fn rebuild_negate(expr: &UntypedExpr, inner: SubstitutionStep) -> SubstitutionStep {
    let mut reduced = expr.clone();
    match &mut reduced {
        UntypedExpr::NegateBool { value, .. } | UntypedExpr::NegateInt { value, .. } => {
            **value = inner.expr;
        }
        _ => unreachable!(),
    }
    SubstitutionStep::new(reduced, inner.note).with_context_opt(inner.context)
}

fn block_from_statements(stmts: Vec<UntypedStatement>) -> Result<UntypedExpr, SubstitutionError> {
    use gleam_core::ast::Statement;
    let stmts = vec1::Vec1::try_from_vec(stmts).expect("Empty block caught by validator");
    if stmts.len() == 1
        && let Statement::Expression(expr) = stmts.first()
    {
        return Ok(expr.clone());
    }
    Ok(UntypedExpr::Block {
        location: S,
        statements: stmts,
    })
}

pub fn is_value(expr: &UntypedExpr) -> bool {
    match expr {
        UntypedExpr::Int { .. } | UntypedExpr::Float { .. } | UntypedExpr::String { .. } => true,

        UntypedExpr::Var { .. } => true,

        UntypedExpr::List { elements, tail, .. } => {
            elements.iter().all(is_value) && tail.as_deref().is_none_or(is_value)
        }

        UntypedExpr::Tuple { elements, .. } => elements.iter().all(is_value),

        UntypedExpr::NegateInt { value, .. } => matches!(value.as_ref(), UntypedExpr::Int { .. }),
        UntypedExpr::NegateBool { value, .. } => {
            matches!(value.as_ref(), UntypedExpr::Var { name, .. } if name == "True" || name == "False")
        }

        _ => false,
    }
}

fn reduce_primitive_expr(expr: &UntypedExpr) -> UntypedExpr {
    match expr {
        UntypedExpr::BinOp {
            name, left, right, ..
        } => {
            let lv = value_from_expr(left);
            match name {
                BinOp::And => match lv {
                    PrimVal::Bool(false) => bool_expr(false),
                    PrimVal::Bool(true) => {
                        let PrimVal::Bool(v) = value_from_expr(right) else {
                            panic!("Invalid operand type");
                        };
                        bool_expr(v)
                    }
                    _ => panic!("Invalid operand type"),
                },
                BinOp::Or => match lv {
                    PrimVal::Bool(true) => bool_expr(true),
                    PrimVal::Bool(false) => {
                        let PrimVal::Bool(v) = value_from_expr(right) else {
                            panic!("Invalid operand type");
                        };
                        bool_expr(v)
                    }
                    _ => panic!("Invalid operand type"),
                },
                _ => {
                    let rv = value_from_expr(right);
                    primitive_bin_op(*name, lv, rv)
                }
            }
        }

        UntypedExpr::NegateBool { value, .. } => {
            let PrimVal::Bool(v) = value_from_expr(value) else {
                panic!("Invalid operand type");
            };
            bool_expr(!v)
        }

        UntypedExpr::NegateInt { value, .. } => match value_from_expr(value) {
            PrimVal::Int(v) => int_expr(-v),
            _ => panic!("Invalid operand type"),
        },

        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PrimVal {
    Int(BigInt),
    Float(f64),
    String(String),
    Bool(bool),
}

fn value_from_expr(expr: &UntypedExpr) -> PrimVal {
    match expr {
        UntypedExpr::Int { int_value, .. } => PrimVal::Int(int_value.clone()),
        UntypedExpr::Float { float_value, .. } => PrimVal::Float(float_value.value()),
        UntypedExpr::String { value, .. } => PrimVal::String(value.to_string()),
        UntypedExpr::Var { name, .. } => match name.as_ref() {
            "True" => PrimVal::Bool(true),
            "False" => PrimVal::Bool(false),
            _ => panic!("Invalid value expression"),
        },
        _ => panic!("Invalid value expression"),
    }
}

fn primitive_bin_op(op: BinOp, left: PrimVal, right: PrimVal) -> UntypedExpr {
    match op {
        BinOp::Eq => bool_expr(left == right),
        BinOp::NotEq => bool_expr(left != right),
        BinOp::LtInt => int_cmp(left, right, |a, b| a < b, op),
        BinOp::LtEqInt => int_cmp(left, right, |a, b| a <= b, op),
        BinOp::GtInt => int_cmp(left, right, |a, b| a > b, op),
        BinOp::GtEqInt => int_cmp(left, right, |a, b| a >= b, op),
        BinOp::LtFloat => float_cmp(left, right, |a, b| a < b, op),
        BinOp::LtEqFloat => float_cmp(left, right, |a, b| a <= b, op),
        BinOp::GtFloat => float_cmp(left, right, |a, b| a > b, op),
        BinOp::GtEqFloat => float_cmp(left, right, |a, b| a >= b, op),
        BinOp::AddInt => int_arith(left, right, |a, b| a + b, op),
        BinOp::SubInt => int_arith(left, right, |a, b| a - b, op),
        BinOp::MultInt => int_arith(left, right, |a, b| a * b, op),
        BinOp::DivInt => int_arith(
            left,
            right,
            |a, b| {
                if b == BigInt::from(0) {
                    BigInt::from(0)
                } else {
                    a / b
                }
            },
            op,
        ),
        BinOp::RemainderInt => int_arith(
            left,
            right,
            |a, b| {
                if b == BigInt::from(0) {
                    BigInt::from(0)
                } else {
                    a % b
                }
            },
            op,
        ),
        BinOp::AddFloat => float_arith(left, right, |a, b| a + b, op),
        BinOp::SubFloat => float_arith(left, right, |a, b| a - b, op),
        BinOp::MultFloat => float_arith(left, right, |a, b| a * b, op),
        BinOp::DivFloat => float_arith(
            left,
            right,
            |a, b| {
                if b == 0.0 { 0.0 } else { a / b }
            },
            op,
        ),
        BinOp::Concatenate => match (left, right) {
            (PrimVal::String(a), PrimVal::String(b)) => string_expr(format!("{a}{b}")),
            _ => panic!("Invalid operand type"),
        },
        BinOp::And | BinOp::Or => unreachable!("handled in reduce_once"),
    }
}

fn int_cmp(
    left: PrimVal,
    right: PrimVal,
    f: impl FnOnce(&BigInt, &BigInt) -> bool,
    _op: BinOp,
) -> UntypedExpr {
    match (left, right) {
        (PrimVal::Int(a), PrimVal::Int(b)) => bool_expr(f(&a, &b)),
        _ => panic!("Invalid operand type"),
    }
}

fn float_cmp(
    left: PrimVal,
    right: PrimVal,
    f: impl FnOnce(f64, f64) -> bool,
    _op: BinOp,
) -> UntypedExpr {
    match (left, right) {
        (PrimVal::Float(a), PrimVal::Float(b)) => bool_expr(f(a, b)),
        _ => panic!("Invalid operand type"),
    }
}

fn int_arith(
    left: PrimVal,
    right: PrimVal,
    f: impl FnOnce(BigInt, BigInt) -> BigInt,
    _op: BinOp,
) -> UntypedExpr {
    match (left, right) {
        (PrimVal::Int(a), PrimVal::Int(b)) => int_expr(f(a, b)),
        _ => panic!("Invalid operand type"),
    }
}

fn float_arith(
    left: PrimVal,
    right: PrimVal,
    f: impl FnOnce(f64, f64) -> f64,
    _op: BinOp,
) -> UntypedExpr {
    match (left, right) {
        (PrimVal::Float(a), PrimVal::Float(b)) => float_expr(f(a, b)),
        _ => panic!("Invalid operand type"),
    }
}

fn reduce_case(expr: &UntypedExpr) -> (UntypedExpr, &'static str) {
    let UntypedExpr::Case {
        subjects, clauses, ..
    } = expr
    else {
        unreachable!();
    };

    let clauses = clauses.as_ref().expect("No clauses in case");
    let first = clauses.first().expect("Empty clauses in case");

    if let Some(env) = match_clause(first, subjects) {
        let result = substitute_expr(&first.then, &env);
        return (result, "select case branch");
    }

    let remaining = clauses[1..].to_vec();
    if remaining.is_empty() {
        panic!("Non-exhaustive patterns caught by validator");
    }

    (
        UntypedExpr::Case {
            location: S,
            subjects: subjects.clone(),
            clauses: Some(remaining),
        },
        "remove first case branch",
    )
}

fn match_clause(
    clause: &Clause<UntypedExpr, (), ()>,
    subjects: &[UntypedExpr],
) -> Option<HashMap<EcoString, UntypedExpr>> {
    if clause.pattern.len() != subjects.len() {
        return None;
    }
    let mut env = HashMap::new();
    for (pattern, subject) in clause.pattern.iter().zip(subjects) {
        if !match_pattern(pattern, subject, &mut env) {
            return None;
        }
    }
    Some(env)
}

fn match_pattern(
    pattern: &Pattern<()>,
    subject: &UntypedExpr,
    env: &mut HashMap<EcoString, UntypedExpr>,
) -> bool {
    match pattern {
        Pattern::Discard { .. } => true,

        Pattern::Variable { name, .. } => {
            env.insert(name.clone(), subject.clone());
            true
        }

        Pattern::Assign { name, pattern, .. } => {
            if match_pattern(pattern, subject, env) {
                env.insert(name.clone(), subject.clone());
                true
            } else {
                false
            }
        }

        Pattern::Int { int_value, .. } => match subject {
            UntypedExpr::Int { int_value: v, .. } => int_value == v,
            _ => false,
        },

        Pattern::Float { float_value, .. } => match subject {
            UntypedExpr::Float { float_value: v, .. } => float_value == v,
            _ => false,
        },

        Pattern::String { value, .. } => match subject {
            UntypedExpr::String { value: v, .. } => value == v,
            _ => false,
        },

        Pattern::List { elements, tail, .. } => match subject {
            UntypedExpr::List { .. } => match_list_pattern(elements, tail.as_deref(), subject, env),
            _ => false,
        },

        Pattern::Tuple { elements, .. } => match subject {
            UntypedExpr::Tuple { elements: vals, .. } => {
                if elements.len() != vals.len() {
                    return false;
                }
                for (p, v) in elements.iter().zip(vals) {
                    if !match_pattern(p, v, env) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        },

        Pattern::Constructor {
            name, arguments, ..
        } => match_constructor_pattern(name, arguments, subject, env),

        _ => panic!("Unsupported pattern should have been caught by validator"),
    }
}

fn match_constructor_pattern(
    ctor_name: &EcoString,
    ctor_args: &[CallArg<Pattern<()>>],
    subject: &UntypedExpr,
    env: &mut HashMap<EcoString, UntypedExpr>,
) -> bool {
    match subject {
        UntypedExpr::Var { name, .. } => ctor_args.is_empty() && name == ctor_name,
        UntypedExpr::Call { fun, arguments, .. } => {
            let UntypedExpr::Var { name, .. } = fun.as_ref() else {
                return false;
            };
            if name != ctor_name || ctor_args.len() != arguments.len() {
                return false;
            }
            if !is_constructor_name(name) {
                return false;
            }
            for (p_arg, v_arg) in ctor_args.iter().zip(arguments) {
                if !match_pattern(&p_arg.value, &v_arg.value, env) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn is_constructor_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

fn match_list_pattern(
    patterns: &[Pattern<()>],
    tail_pattern: Option<&TailPattern<()>>,
    subject: &UntypedExpr,
    env: &mut HashMap<EcoString, UntypedExpr>,
) -> bool {
    let UntypedExpr::List { elements, tail, .. } = subject else {
        return false;
    };

    if elements.len() < patterns.len() {
        return false;
    }

    for (p, e) in patterns.iter().zip(elements) {
        if !match_pattern(p, e, env) {
            return false;
        }
    }

    match tail_pattern {
        None => elements.len() == patterns.len() && tail.is_none(),
        Some(tail_pat) => {
            let rest = UntypedExpr::List {
                location: S,
                elements: elements[patterns.len()..].to_vec(),
                tail: tail.clone(),
            };
            match_pattern(&tail_pat.pattern, &rest, env)
        }
    }
}

fn collect_pattern_bindings(
    pattern: &Pattern<()>,
    value: &UntypedExpr,
    env: &mut HashMap<EcoString, UntypedExpr>,
) {
    if let Pattern::Variable { name, .. } = pattern {
        env.insert(name.clone(), value.clone());
        return;
    }
    match (pattern, value) {
        (Pattern::Tuple { elements: pats, .. }, UntypedExpr::Tuple { elements: vals, .. }) => {
            for (p, v) in pats.iter().zip(vals) {
                collect_pattern_bindings(p, v, env);
            }
        }
        (
            Pattern::List {
                elements: pats,
                tail: tail_pat,
                ..
            },
            UntypedExpr::List {
                elements: vals,
                tail: tail_val,
                ..
            },
        ) => {
            for (p, v) in pats.iter().zip(vals) {
                collect_pattern_bindings(p, v, env);
            }
            if let Some(tail_pat) = tail_pat {
                let rest = UntypedExpr::List {
                    location: S,
                    elements: vals[pats.len()..].to_vec(),
                    tail: tail_val.clone(),
                };
                collect_pattern_bindings(&tail_pat.pattern, &rest, env);
            }
        }
        (Pattern::Assign { name, pattern, .. }, _) => {
            env.insert(name.clone(), value.clone());
            collect_pattern_bindings(pattern, value, env);
        }
        (
            Pattern::Constructor {
                name: pat_name,
                arguments: pat_args,
                ..
            },
            UntypedExpr::Call { fun, arguments, .. },
        ) => {
            if let UntypedExpr::Var { name: val_name, .. } = fun.as_ref()
                && pat_name == val_name
                && pat_args.len() == arguments.len()
            {
                for (p_arg, v_arg) in pat_args.iter().zip(arguments) {
                    collect_pattern_bindings(&p_arg.value, &v_arg.value, env);
                }
            }
        }
        (Pattern::Discard { .. }, _) => {}
        _ => {}
    }
}

pub fn int_expr(value: BigInt) -> UntypedExpr {
    let text = value.to_string();
    UntypedExpr::Int {
        location: S,
        value: text.into(),
        int_value: value,
    }
}

pub fn float_expr(value: f64) -> UntypedExpr {
    let text = float_to_source(value);
    let float_value = gleam_core::parse::LiteralFloatValue::parse(&text)
        .expect("Invalid float value should have been caught by parser");
    UntypedExpr::Float {
        location: S,
        value: text.into(),
        float_value,
    }
}

pub fn bool_expr(value: bool) -> UntypedExpr {
    UntypedExpr::Var {
        location: S,
        name: if value { "True" } else { "False" }.into(),
    }
}

pub fn string_expr(value: String) -> UntypedExpr {
    UntypedExpr::String {
        location: S,
        value: value.into(),
    }
}

fn float_to_source(value: f64) -> String {
    let mut text = value.to_string();
    if !text.contains(['.', 'e', 'E']) {
        text.push_str(".0");
    }
    text
}

fn is_bool_lit(expr: &UntypedExpr, expected: bool) -> bool {
    matches!(expr, UntypedExpr::Var { name, .. } if name.as_ref() == if expected { "True" } else { "False" })
}

pub fn substitute_expr(expr: &UntypedExpr, env: &HashMap<EcoString, UntypedExpr>) -> UntypedExpr {
    let mut res = expr.clone();
    match &mut res {
        UntypedExpr::Var { name, .. } => {
            if let Some(replacement) = env.get(name) {
                return replacement.clone();
            }
        }
        UntypedExpr::Call { fun, arguments, .. } => {
            **fun = substitute_expr(fun, env);
            for arg in arguments {
                arg.value = substitute_expr(&arg.value, env);
            }
        }
        UntypedExpr::BinOp { left, right, .. } => {
            **left = substitute_expr(left, env);
            **right = substitute_expr(right, env);
        }
        UntypedExpr::NegateBool { value, .. } | UntypedExpr::NegateInt { value, .. } => {
            **value = substitute_expr(value, env);
        }
        UntypedExpr::List { elements, tail, .. } => {
            for e in elements {
                *e = substitute_expr(e, env);
            }
            if let Some(t) = tail {
                **t = substitute_expr(t, env);
            }
        }
        UntypedExpr::Tuple { elements, .. } => {
            for e in elements {
                *e = substitute_expr(e, env);
            }
        }
        UntypedExpr::Block { statements, .. } => {
            for s in statements {
                *s = substitute_statement(s, env);
            }
        }
        UntypedExpr::Case {
            subjects, clauses, ..
        } => {
            for s in subjects {
                *s = substitute_expr(s, env);
            }
            if let Some(clauses) = clauses {
                for clause in clauses {
                    let mut inner = env.clone();
                    for pat in &clause.pattern {
                        remove_pattern_names(pat, &mut inner);
                    }
                    clause.then = substitute_expr(&clause.then, &inner);
                }
            }
        }
        _ => {}
    }
    res
}

fn substitute_statement(
    stmt: &UntypedStatement,
    env: &HashMap<EcoString, UntypedExpr>,
) -> UntypedStatement {
    let mut res = stmt.clone();
    match &mut res {
        Statement::Expression(expr) => *expr = substitute_expr(expr, env),
        Statement::Assignment(assignment) => {
            assignment.value = substitute_expr(&assignment.value, env);
        }
        _ => {}
    }
    res
}

fn remove_pattern_names(pattern: &Pattern<()>, env: &mut HashMap<EcoString, UntypedExpr>) {
    match pattern {
        Pattern::Variable { name, .. } => {
            env.remove(name);
        }
        Pattern::Assign { name, pattern, .. } => {
            env.remove(name);
            remove_pattern_names(pattern, env);
        }
        Pattern::List { elements, tail, .. } => {
            for e in elements {
                remove_pattern_names(e, env);
            }
            if let Some(t) = tail {
                remove_pattern_names(&t.pattern, env);
            }
        }
        Pattern::Tuple { elements, .. } => {
            for e in elements {
                remove_pattern_names(e, env);
            }
        }
        Pattern::Constructor { arguments, .. } => {
            for arg in arguments {
                remove_pattern_names(&arg.value, env);
            }
        }
        _ => {}
    }
}

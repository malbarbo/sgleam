use gleam_core::ast::UntypedExpr;

use super::SubstitutionError;
use crate::format::format_source;

/// Renders an `UntypedExpr` to formatted Gleam source text.
///
/// Wraps the expression in a `pub fn main() { ... }` module, formats it using
/// the Gleam formatter, then strips the wrapper.
pub fn render_expr(expr: &UntypedExpr) -> Result<String, SubstitutionError> {
    let raw = raw_render_expr(expr);
    format_expr_source(&raw)
}

/// Renders a full function definition.
pub fn render_function(
    name: &str,
    arguments: &[ecow::EcoString],
    body: &UntypedExpr,
) -> Result<String, SubstitutionError> {
    let args_str = arguments
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let body_str = raw_render_expr(body);
    let raw = format!("pub fn {name}({args_str}) {{\n{body_str}\n}}\n");
    format_source(&raw)
        .map(|s| s.trim().to_string())
        .map_err(|_| SubstitutionError::FormattingError)
}

fn format_expr_source(source: &str) -> Result<String, SubstitutionError> {
    let wrapped = format!("pub fn main() {{\n{source}\n}}\n");
    let formatted = format_source(&wrapped)?;
    let prefix = "pub fn main() {\n";
    let suffix = "\n}\n";
    let body = formatted
        .strip_prefix(prefix)
        .and_then(|body| body.strip_suffix(suffix))
        .ok_or(SubstitutionError::FormattingError)?;

    Ok(body
        .lines()
        .map(|line: &str| line.strip_prefix("  ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn raw_render_expr(expr: &UntypedExpr) -> String {
    match expr {
        UntypedExpr::Int { value, .. } => value.to_string(),
        UntypedExpr::Float { value, .. } => value.to_string(),
        UntypedExpr::String { value, .. } => format!("\"{}\"", escape_string(value)),
        UntypedExpr::Var { name, .. } => name.to_string(),

        UntypedExpr::NegateInt { value, .. } => {
            let inner = raw_render_child(value, Prec::Prefix);
            format!("-{inner}")
        }
        UntypedExpr::NegateBool { value, .. } => {
            let inner = raw_render_child(value, Prec::Prefix);
            format!("!{inner}")
        }

        UntypedExpr::BinOp {
            name, left, right, ..
        } => {
            let prec = bin_op_prec(*name);
            let left = raw_render_bin_child(left, prec, Side::Left);
            let right = raw_render_bin_child(right, prec, Side::Right);
            format!("{left} {} {right}", name.name())
        }

        UntypedExpr::Call { fun, arguments, .. } => {
            let fun = raw_render_expr(fun);
            let args = arguments
                .iter()
                .map(|arg| {
                    let value = raw_render_expr(&arg.value);
                    if let Some(label) = &arg.label {
                        format!("{label}: {value}")
                    } else {
                        value
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{fun}({args})")
        }

        UntypedExpr::List { elements, tail, .. } => {
            let mut parts: Vec<String> = elements.iter().map(raw_render_expr).collect();
            if let Some(tail) = tail {
                parts.push(format!("..{}", raw_render_expr(tail)));
            }
            format!("[{}]", parts.join(", "))
        }

        UntypedExpr::Tuple { elements, .. } => {
            let parts: Vec<String> = elements.iter().map(raw_render_expr).collect();
            format!("#({})", parts.join(", "))
        }

        UntypedExpr::Case {
            subjects, clauses, ..
        } => {
            let subjects: Vec<String> = subjects.iter().map(raw_render_expr).collect();
            let clauses_str = clauses
                .as_ref()
                .map(|clauses| {
                    clauses
                        .iter()
                        .map(raw_render_clause)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            format!("case {} {{\n{clauses_str}\n}}", subjects.join(", "))
        }

        UntypedExpr::Block { statements, .. } => {
            let stmts: Vec<String> = statements.iter().map(raw_render_statement).collect();
            if stmts.len() == 1 {
                stmts.into_iter().next().unwrap()
            } else {
                format!("{{\n{}\n}}", stmts.join("\n"))
            }
        }

        _ => "todo /* unsupported expression */".to_string(),
    }
}

fn raw_render_statement(stmt: &gleam_core::ast::UntypedStatement) -> String {
    use gleam_core::ast::Statement;
    match stmt {
        Statement::Expression(expr) => raw_render_expr(expr),
        Statement::Assignment(assignment) => {
            let pattern = raw_render_pattern(&assignment.pattern);
            let value = raw_render_expr(&assignment.value);
            format!("let {pattern} = {value}")
        }
        _ => "todo /* unsupported statement */".to_string(),
    }
}

fn raw_render_clause(clause: &gleam_core::ast::UntypedClause) -> String {
    let patterns: Vec<String> = clause.pattern.iter().map(raw_render_pattern).collect();
    let then = raw_render_expr(&clause.then);
    format!("  {} -> {then}", patterns.join(", "))
}

fn raw_render_pattern(pattern: &gleam_core::ast::UntypedPattern) -> String {
    use gleam_core::ast::Pattern;
    match pattern {
        Pattern::Int { value, .. } => value.to_string(),
        Pattern::Float { value, .. } => value.to_string(),
        Pattern::String { value, .. } => format!("\"{}\"", escape_string(value)),
        Pattern::Variable { name, .. } => name.to_string(),
        Pattern::Discard { name, .. } => name.to_string(),
        Pattern::Assign { name, pattern, .. } => {
            format!("{} as {name}", raw_render_pattern(pattern))
        }
        Pattern::List { elements, tail, .. } => {
            let mut parts: Vec<String> = elements.iter().map(raw_render_pattern).collect();
            if let Some(tail) = tail {
                parts.push(format!("..{}", raw_render_pattern(&tail.pattern)));
            }
            format!("[{}]", parts.join(", "))
        }
        Pattern::Tuple { elements, .. } => {
            let parts: Vec<String> = elements.iter().map(raw_render_pattern).collect();
            format!("#({})", parts.join(", "))
        }
        Pattern::Constructor {
            name,
            arguments,
            module,
            ..
        } => {
            let qualified = if let Some((module, _)) = module {
                format!("{module}.{name}")
            } else {
                name.to_string()
            };
            if arguments.is_empty() {
                qualified
            } else {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| raw_render_pattern(&a.value))
                    .collect();
                format!("{qualified}({})", args.join(", "))
            }
        }
        _ => "_".to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    Lowest,
    Or,
    And,
    Compare,
    Concat,
    AddSub,
    MulDiv,
    Prefix,
    Call,
    Atomic,
}

#[derive(PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

fn expr_prec(expr: &UntypedExpr) -> Prec {
    match expr {
        UntypedExpr::Int { .. }
        | UntypedExpr::Float { .. }
        | UntypedExpr::String { .. }
        | UntypedExpr::Var { .. }
        | UntypedExpr::List { .. }
        | UntypedExpr::Tuple { .. } => Prec::Atomic,
        UntypedExpr::Call { .. } => Prec::Call,
        UntypedExpr::NegateInt { .. } | UntypedExpr::NegateBool { .. } => Prec::Prefix,
        UntypedExpr::BinOp { name, .. } => bin_op_prec(*name),
        UntypedExpr::Case { .. } | UntypedExpr::Block { .. } => Prec::Lowest,
        _ => Prec::Lowest,
    }
}

fn bin_op_prec(op: gleam_core::ast::BinOp) -> Prec {
    use gleam_core::ast::BinOp;
    match op {
        BinOp::Or => Prec::Or,
        BinOp::And => Prec::And,
        BinOp::Eq
        | BinOp::NotEq
        | BinOp::LtInt
        | BinOp::LtEqInt
        | BinOp::GtInt
        | BinOp::GtEqInt
        | BinOp::LtFloat
        | BinOp::LtEqFloat
        | BinOp::GtFloat
        | BinOp::GtEqFloat => Prec::Compare,
        BinOp::Concatenate => Prec::Concat,
        BinOp::AddInt | BinOp::AddFloat | BinOp::SubInt | BinOp::SubFloat => Prec::AddSub,
        BinOp::MultInt
        | BinOp::MultFloat
        | BinOp::DivInt
        | BinOp::DivFloat
        | BinOp::RemainderInt => Prec::MulDiv,
    }
}

fn raw_render_child(expr: &UntypedExpr, min_prec: Prec) -> String {
    let rendered = raw_render_expr(expr);
    if expr_prec(expr) < min_prec {
        format!("{{ {rendered} }}")
    } else {
        rendered
    }
}

fn raw_render_bin_child(expr: &UntypedExpr, parent_prec: Prec, side: Side) -> String {
    let child_prec = expr_prec(expr);
    let rendered = raw_render_expr(expr);
    let needs_wrap = child_prec < parent_prec || (side == Side::Right && child_prec == parent_prec);
    if needs_wrap {
        format!("{{ {rendered} }}")
    } else {
        rendered
    }
}

fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c => result.push(c),
        }
    }
    result
}

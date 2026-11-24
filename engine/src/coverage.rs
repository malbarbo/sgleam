//! The rewrite of a Gleam source that marks where each region of the code
//! starts, so a run can tell which regions ran.
//!
//! The rewrite inserts text and never a line break, except before the import
//! at the end, so every line of the source keeps its number.

use camino::Utf8PathBuf;
use ecow::{EcoString, eco_format};
use gleam_core::{
    ast::{
        AssignmentKind, BinOp, Definition, FunctionLiteralKind, Statement, TodoKind, UntypedExpr,
        UntypedStatement,
    },
    build::Target,
    warning::VectorWarningEmitterIO,
};
use src_span::SrcSpan;
use std::rc::Rc;

use crate::{
    error::SgleamError,
    gleam::{self, Built, Project},
    scope::mentioned,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// The body of a function, named or anonymous.
    Function,
    /// The expression of a `case` clause.
    Clause,
    /// The right operand of `&&` or `||`.
    Operand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub kind: RegionKind,
    /// The span in the original source. A function spans its head and its
    /// body, and a clause spans its patterns and its expression.
    pub span: SrcSpan,
    /// The name of the top level function that holds the region.
    pub function: EcoString,
    /// Whether the region ran. Only a run with coverage sets it.
    pub ran: bool,
}

#[derive(Debug)]
pub struct Rewrite {
    pub source: String,
    /// The region that `hit(first + i)` marks is `regions[i]`.
    pub regions: Vec<Region>,
}

/// A file of the user, with its coverage regions.
#[derive(Debug)]
pub struct File {
    /// The path the user gave.
    pub path: Utf8PathBuf,
    /// The source before the rewrite. The spans of the regions are offsets
    /// into it.
    pub source: EcoString,
    /// The numbers of the marks continue from the last region of the file
    /// before, so the first region of the first file is the mark 0.
    pub regions: Vec<Region>,
}

/// Rewrites the modules that `paths` became in `built` with the coverage
/// marks, and compiles `project` again. The compile drops its warnings, as
/// the first compile already showed them for the source the user wrote.
pub fn compile(
    project: &mut Project,
    paths: &[Utf8PathBuf],
    built: &Built,
) -> Result<Vec<File>, SgleamError> {
    let mut files = vec![];
    let mut first = 0;
    for (index, path) in paths.iter().enumerate() {
        let Some(module) = built.module(index) else {
            continue;
        };
        let name = &module.name;
        let rewrite = rewrite(path.clone(), &module.code, first)?;
        first += rewrite.regions.len();
        project.rewrite_source(name, &rewrite.source)?;
        files.push(File {
            path: path.clone(),
            source: module.code.clone(),
            regions: rewrite.regions,
        });
    }

    project
        .compile_with_modules(
            Rc::new(VectorWarningEmitterIO::new()),
            &mut imbl::HashMap::new(),
            &mut imbl::HashMap::new(),
        )
        .map_err(|err| {
            // The error names the rewritten source, which the user never
            // wrote, so only the log has it.
            tracing::debug!("coverage compile: {err:?}");
            SgleamError::CoverageCompile
        })?;
    Ok(files)
}

/// Rewrites `src` so that each region calls `hit`, `hit_true` or
/// `hit_false` of `sgleam/coverage` when it starts. The numbers start at
/// `first`, so the files of one program can share a single set of numbers.
pub fn rewrite(path: Utf8PathBuf, src: &str, first: usize) -> Result<Rewrite, gleam_core::Error> {
    let parsed = gleam::parse(path, src)?;

    let mut marker = Marker {
        src,
        alias: free_alias(src),
        first,
        function: EcoString::new(),
        regions: vec![],
        inserts: vec![],
    };
    for definition in &parsed.module.definitions {
        // A function that never runs on JavaScript would always show as not
        // run.
        if let Definition::Function(function) = &definition.definition
            && definition.is_for(Target::JavaScript)
            && function.external_javascript.is_none()
        {
            marker.function = function
                .name
                .as_ref()
                .map(|(_, name)| name.clone())
                .unwrap_or_default();
            let span = SrcSpan::new(function.location.start, function.end_position);
            marker.body(span, &function.body);
        }
    }

    if marker.regions.is_empty() {
        return Ok(Rewrite {
            source: src.into(),
            regions: vec![],
        });
    }

    // The sort is stable, so at one offset the text keeps the order of the
    // walk, which opens the outer block before the inner one and closes the
    // inner block first.
    let mut inserts = marker.inserts;
    inserts.sort_by_key(|(at, _)| *at);
    let mut source = String::with_capacity(src.len() + inserts.len() * 32);
    let mut last = 0;
    for (at, text) in inserts {
        source.push_str(&src[last..at as usize]);
        source.push_str(&text);
        last = at as usize;
    }
    source.push_str(&src[last..]);
    // A file that ends in a comment with no line break would swallow the
    // import.
    source.push_str(&format!("\nimport sgleam/coverage as {}\n", marker.alias));

    Ok(Rewrite {
        source,
        regions: marker.regions,
    })
}

/// A name for the import of `sgleam/coverage` that no name in `src` uses, as
/// a local, a function or a module with that name would hide the import.
fn free_alias(src: &str) -> EcoString {
    let names = mentioned(src);
    let mut alias = EcoString::from("sgleam_coverage");
    let mut n = 1;
    while names.contains(&alias) {
        alias = eco_format!("sgleam_coverage_{n}");
        n += 1;
    }
    alias
}

struct Marker<'a> {
    src: &'a str,
    alias: EcoString,
    first: usize,
    function: EcoString,
    regions: Vec<Region>,
    inserts: Vec<(u32, String)>,
}

impl Marker<'_> {
    fn region(&mut self, kind: RegionKind, span: SrcSpan) -> usize {
        self.regions.push(Region {
            kind,
            span,
            function: self.function.clone(),
            ran: false,
        });
        self.first + self.regions.len() - 1
    }

    fn insert(&mut self, at: u32, text: String) {
        self.inserts.push((at, text));
    }

    fn body(&mut self, span: SrcSpan, statements: &[UntypedStatement]) {
        // An empty body holds the `todo` the parser adds, and a mark before it
        // would make the function return `Nil`.
        let empty = matches!(
            statements,
            [Statement::Expression(UntypedExpr::Todo {
                kind: TodoKind::EmptyFunction { .. } | TodoKind::EmptyBlock,
                ..
            })]
        );
        if let Some(statement) = statements.first()
            && !empty
        {
            let n = self.region(RegionKind::Function, span);
            self.insert(
                statement.start_byte_index(),
                format!("{}.hit({n}.0) ", self.alias),
            );
            if let Statement::Expression(expression) = statement {
                self.leading_minus(expression);
            }
        }
        self.statements(statements);
    }

    fn statements(&mut self, statements: &[UntypedStatement]) {
        for statement in statements {
            self.statement(statement);
        }
    }

    /// After a mark, a leading `-` reads as a subtraction from the mark, so
    /// the negated value goes in a block of its own: `hit(1) { -x } * 2`.
    fn leading_minus(&mut self, expression: &UntypedExpr) {
        let start = expression.start_byte_index();
        if self.src.as_bytes().get(start as usize) != Some(&b'-') {
            return;
        }
        let mut node = expression;
        loop {
            node = match node {
                UntypedExpr::BinOp { left, .. } => left,
                UntypedExpr::Pipeline { expressions } => expressions.first(),
                UntypedExpr::Call { fun, .. } => fun,
                UntypedExpr::FieldAccess { container, .. } => container,
                UntypedExpr::TupleIndex { tuple, .. } => tuple,
                UntypedExpr::NegateInt { location, .. }
                | UntypedExpr::Int { location, .. }
                | UntypedExpr::Float { location, .. } => {
                    // Every mark inside the negated value comes before its
                    // last token, so this brace closes after all of them.
                    self.insert(location.start, "{ ".into());
                    self.insert(location.end, " }".into());
                    return;
                }
                _ => return,
            };
        }
    }

    fn statement(&mut self, statement: &UntypedStatement) {
        match statement {
            Statement::Expression(expression) => self.expression(expression),
            Statement::Assignment(assignment) => {
                self.expression(&assignment.value);
                if let AssignmentKind::Assert {
                    message: Some(message),
                    ..
                } = &assignment.kind
                {
                    self.expression(message);
                }
            }
            Statement::Use(use_) => self.expression(&use_.call),
            Statement::Assert(assert) => {
                self.expression(&assert.value);
                if let Some(message) = &assert.message {
                    self.expression(message);
                }
            }
        }
    }

    fn expression(&mut self, expression: &UntypedExpr) {
        match expression {
            UntypedExpr::Int { .. }
            | UntypedExpr::Float { .. }
            | UntypedExpr::String { .. }
            | UntypedExpr::Var { .. } => {}

            UntypedExpr::Block { statements, .. } => {
                self.statements(statements);
            }

            UntypedExpr::Fn {
                location,
                kind,
                body,
                ..
            } => match kind {
                FunctionLiteralKind::Anonymous { .. } => self.body(*location, body),
                // A capture has no body in the source. Its body is the call
                // that it replaces.
                FunctionLiteralKind::Capture { .. } | FunctionLiteralKind::Use { .. } => {
                    self.statements(body);
                }
            },

            UntypedExpr::List { elements, tail, .. } => {
                for element in elements {
                    self.expression(element);
                }
                if let Some(tail) = tail {
                    self.expression(tail);
                }
            }

            UntypedExpr::Call { fun, arguments, .. } => {
                self.expression(fun);
                for argument in arguments {
                    self.expression(&argument.value);
                }
            }

            UntypedExpr::BinOp {
                operator,
                left,
                right,
                ..
            } => {
                self.expression(left);
                if matches!(operator, BinOp::And | BinOp::Or) {
                    // The mark runs before the operand, so an operand that
                    // panics still shows as run.
                    let location = right.location();
                    let n = self.region(RegionKind::Operand, location);
                    let (mark, op) = match operator {
                        BinOp::And => ("hit_true", "&&"),
                        _ => ("hit_false", "||"),
                    };
                    self.insert(
                        location.start,
                        format!("{}.{mark}({n}.0) {op} ", self.alias),
                    );
                    self.expression(right);
                } else {
                    self.expression(right);
                }
            }

            UntypedExpr::Pipeline { expressions } => {
                for expression in expressions {
                    self.expression(expression);
                }
            }

            UntypedExpr::Case {
                subjects, clauses, ..
            } => {
                for subject in subjects {
                    self.expression(subject);
                }
                for clause in clauses.iter().flatten() {
                    let n = self.region(RegionKind::Clause, clause.location);
                    let then = clause.then.location();
                    self.insert(then.start, format!("{{ {}.hit({n}.0) ", self.alias));
                    self.leading_minus(&clause.then);
                    self.expression(&clause.then);
                    self.insert(then.end, " }".into());
                }
            }

            UntypedExpr::FieldAccess { container, .. } => self.expression(container),

            UntypedExpr::Tuple { elements, .. } => {
                for element in elements {
                    self.expression(element);
                }
            }

            UntypedExpr::TupleIndex { tuple, .. } => self.expression(tuple),

            UntypedExpr::Todo { message, .. } | UntypedExpr::Panic { message, .. } => {
                if let Some(message) = message {
                    self.expression(message);
                }
            }

            UntypedExpr::Echo {
                expression,
                message,
                ..
            } => {
                if let Some(expression) = expression {
                    self.expression(expression);
                }
                if let Some(message) = message {
                    self.expression(message);
                }
            }

            UntypedExpr::BitArray { segments, .. } => {
                for segment in segments {
                    self.expression(&segment.value);
                }
            }

            UntypedExpr::RecordUpdate {
                constructor,
                record,
                arguments,
                ..
            } => {
                self.expression(constructor);
                self.expression(&record.base);
                for argument in arguments {
                    self.expression(&argument.value);
                }
            }

            UntypedExpr::NegateBool { value, .. } | UntypedExpr::NegateInt { value, .. } => {
                self.expression(value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite_ok(src: &str, first: usize) -> Rewrite {
        rewrite("/src/a.gleam".into(), src, first).expect("parse the source")
    }

    fn rewritten(src: &str) -> String {
        let rewrite = rewrite_ok(src, 0);
        // The rewritten source has to parse, and every line keeps its number.
        gleam::parse("/src/a.gleam".into(), &rewrite.source).expect("parse the rewritten source");
        let import = "\nimport sgleam/coverage as sgleam_coverage\n";
        let source = rewrite
            .source
            .strip_suffix(import)
            .unwrap_or(&rewrite.source);
        assert_eq!(source.lines().count(), src.lines().count());
        source.replace("sgleam_coverage.", "")
    }

    fn regions(src: &str) -> Vec<(RegionKind, &str, EcoString)> {
        rewrite_ok(src, 0)
            .regions
            .into_iter()
            .map(|region| {
                let text = &src[region.span.start as usize..region.span.end as usize];
                (region.kind, text, region.function)
            })
            .collect()
    }

    #[test]
    fn function_body() {
        assert_eq!(rewritten("fn f(x) { x * 2 }"), "fn f(x) { hit(0.0) x * 2 }");
    }

    #[test]
    fn function_body_with_many_statements() {
        assert_eq!(
            rewritten("fn f(x) {\n  let y = x\n  y\n}"),
            "fn f(x) {\n  hit(0.0) let y = x\n  y\n}"
        );
    }

    #[test]
    fn function_body_after_a_comment() {
        assert_eq!(
            rewritten("fn f(x) { // {\n  x\n}"),
            "fn f(x) { // {\n  hit(0.0) x\n}"
        );
    }

    #[test]
    fn empty_body() {
        let rewrite = rewrite_ok("fn f() {}", 0);
        assert_eq!(rewrite.source, "fn f() {}");
        assert!(rewrite.regions.is_empty());
    }

    #[test]
    fn external_function() {
        let src = "@external(javascript, \"./a.mjs\", \"f\")\nfn f() -> Int";
        let rewrite = rewrite_ok(src, 0);
        assert_eq!(rewrite.source, src);
    }

    #[test]
    fn anonymous_function() {
        assert_eq!(
            rewritten("fn f() { fn(x) { x } }"),
            "fn f() { hit(0.0) fn(x) { hit(1.0) x } }"
        );
    }

    #[test]
    fn empty_anonymous_function() {
        assert_eq!(
            rewritten("fn f() { fn() {} }"),
            "fn f() { hit(0.0) fn() {} }"
        );
    }

    #[test]
    fn capture() {
        assert_eq!(
            rewritten("fn f() { g(_, a && b) }"),
            "fn f() { hit(0.0) g(_, a && hit_true(1.0) && b) }"
        );
    }

    #[test]
    fn case_clauses() {
        assert_eq!(
            rewritten("fn f(n) {\n  case n {\n    0 -> a\n    _ -> b\n  }\n}"),
            "fn f(n) {\n  hit(0.0) case n {\n    0 -> { hit(1.0) a }\n    _ -> { hit(2.0) b }\n  }\n}"
        );
    }

    #[test]
    fn case_clause_with_a_block() {
        assert_eq!(
            rewritten("fn f(n) { case n { _ -> { a b } } }"),
            "fn f(n) { hit(0.0) case n { _ -> { hit(1.0) { a b } } } }"
        );
    }

    #[test]
    fn case_clause_with_alternatives_and_a_guard() {
        assert_eq!(
            rewritten("fn f(n) { case n { 1 | 2 if a && b -> c } }"),
            "fn f(n) { hit(0.0) case n { 1 | 2 if a && b -> { hit(1.0) c } } }"
        );
    }

    #[test]
    fn nested_case() {
        assert_eq!(
            rewritten("fn f(n) { case n { _ -> case n { _ -> a } } }"),
            "fn f(n) { hit(0.0) case n { _ -> { hit(1.0) case n { _ -> { hit(2.0) a } } } } }"
        );
    }

    #[test]
    fn and_or() {
        assert_eq!(
            rewritten("fn f() { a && b || c }"),
            "fn f() { hit(0.0) a && hit_true(1.0) && b || hit_false(2.0) || c }"
        );
    }

    #[test]
    fn and_with_a_right_operand_that_is_an_and() {
        assert_eq!(
            rewritten("fn f() { a || b && c }"),
            "fn f() { hit(0.0) a || hit_false(1.0) || b && hit_true(2.0) && c }"
        );
    }

    #[test]
    fn and_in_a_clause() {
        assert_eq!(
            rewritten("fn f() { case x { _ -> a && b } }"),
            "fn f() { hit(0.0) case x { _ -> { hit(1.0) a && hit_true(2.0) && b } } }"
        );
    }

    #[test]
    fn a_function_for_another_target_or_external() {
        let src = "@target(erlang)\nfn g() { 1 }\n@external(javascript, \"./x.mjs\", \"h\")\nfn h() -> Int { 2 }";
        assert_eq!(rewrite_ok(src, 0).source, src);
    }

    #[test]
    fn and_in_an_assert() {
        assert_eq!(
            rewritten("fn f() { assert a && g(b || c) }"),
            "fn f() { hit(0.0) assert a && hit_true(1.0) && g(b || hit_false(2.0) || c) }"
        );
    }

    #[test]
    fn leading_minus_in_a_body() {
        assert_eq!(
            rewritten("fn f(x) { -x * 2 }"),
            "fn f(x) { hit(0.0) { -x } * 2 }"
        );
    }

    #[test]
    fn leading_minus_literal() {
        assert_eq!(
            rewritten("fn f() { -1 + 2 }"),
            "fn f() { hit(0.0) { -1 } + 2 }"
        );
    }

    #[test]
    fn leading_minus_in_a_clause() {
        assert_eq!(
            rewritten("fn f(x) { case x { _ -> -x } }"),
            "fn f(x) { hit(0.0) case x { _ -> { hit(1.0) { -x } } } }"
        );
    }

    #[test]
    fn minus_not_at_the_start() {
        assert_eq!(rewritten("fn f(x) { 1 - x }"), "fn f(x) { hit(0.0) 1 - x }");
    }

    #[test]
    fn use_() {
        assert_eq!(
            rewritten("fn f() {\n  use x <- g(a && b)\n  x\n}"),
            "fn f() {\n  hit(0.0) use x <- g(a && hit_true(1.0) && b)\n  x\n}"
        );
    }

    #[test]
    fn constants_have_no_marks() {
        let src = "const a = True && False";
        assert_eq!(rewrite_ok(src, 0).source, src);
    }

    #[test]
    fn import_at_the_end() {
        let src = "fn f() { 1 } // end";
        assert_eq!(
            rewrite_ok(src, 0).source,
            "fn f() { sgleam_coverage.hit(0.0) 1 } // end\nimport sgleam/coverage as sgleam_coverage\n"
        );
    }

    #[test]
    fn alias_in_use() {
        let src = "import sgleam_coverage\nfn f(sgleam_coverage_1) { 1 }";
        assert_eq!(
            rewrite_ok(src, 0).source,
            "import sgleam_coverage\nfn f(sgleam_coverage_1) { sgleam_coverage_2.hit(0.0) 1 }\nimport sgleam/coverage as sgleam_coverage_2\n"
        );
    }

    #[test]
    fn first_number() {
        let rewrite = rewrite_ok("fn f() { a && b }", 7);
        assert!(
            rewrite
                .source
                .contains("hit(7.0) a && sgleam_coverage.hit_true(8.0) && b")
        );
    }

    #[test]
    fn regions_have_spans_of_the_original() {
        let src = "fn f(n) { case n { 0 -> a && b } }\nfn g() { fn() { 1 } }";
        assert_eq!(
            regions(src),
            vec![
                (
                    RegionKind::Function,
                    "fn f(n) { case n { 0 -> a && b } }",
                    "f".into()
                ),
                (RegionKind::Clause, "0 -> a && b", "f".into()),
                (RegionKind::Operand, "b", "f".into()),
                (RegionKind::Function, "fn g() { fn() { 1 } }", "g".into()),
                (RegionKind::Function, "fn() { 1 }", "g".into()),
            ]
        );
    }

    #[test]
    fn non_ascii_text() {
        assert_eq!(
            rewritten("fn f() { \"ação\" == \"ã\" && b }"),
            "fn f() { hit(0.0) \"ação\" == \"ã\" && hit_true(1.0) && b }"
        );
    }
}

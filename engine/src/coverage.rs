//! The rewrite of a Gleam source that marks where each region of the code
//! starts, so a run can tell which regions ran.
//!
//! The rewrite inserts text and never a line break, except before the import
//! at the end, so every line of the source keeps its number.

use camino::{Utf8Path, Utf8PathBuf};
use ecow::{EcoString, eco_format};
use gleam_core::{
    ast::{
        AssignmentKind, BinOp, Definition, FunctionLiteralKind, Statement, TodoKind, UntypedExpr,
        UntypedStatement,
    },
    build::{Module, Target},
    warning::VectorWarningEmitterIO,
};
use src_span::{LineNumbers, SrcSpan};
use std::{io::Write, rc::Rc};
use termcolor::{Buffer, Color, ColorSpec, WriteColor};
use unicode_width::UnicodeWidthChar;

use crate::{
    engine::{MainFunction, SMAIN},
    error::SgleamError,
    gleam::{self, Project},
    scope::mentioned,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// The body of a top level function.
    Function,
    /// The body of an anonymous function.
    AnonymousFunction,
    /// The expression of a `case` clause.
    Clause,
    /// The right operand of `&&`, which runs only when `left` is true.
    And { left: SrcSpan },
    /// The right operand of `||`, which runs only when `left` is false.
    Or { left: SrcSpan },
}

impl RegionKind {
    /// The name of the kind, without the span of a left operand.
    pub fn name(&self) -> &'static str {
        match self {
            RegionKind::Function | RegionKind::AnonymousFunction => "function",
            RegionKind::Clause => "clause",
            RegionKind::And { .. } => "and",
            RegionKind::Or { .. } => "or",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub kind: RegionKind,
    /// The span in the original source. A function spans its head and its
    /// body, and a clause spans its patterns and its expression.
    pub span: SrcSpan,
    /// The part of the span that a report shows. A function shows its head,
    /// and a clause or an operand shows all of its span.
    pub shown: SrcSpan,
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

/// Rewrites `modules` with the coverage marks, and compiles `project` again.
/// Each module comes with the path the user gave for it. The compile drops
/// its warnings, as the first compile already showed them for the source the
/// user wrote.
pub fn compile(
    project: &mut Project,
    modules: &[(&Utf8Path, &Module)],
) -> Result<Vec<File>, SgleamError> {
    let mut files = vec![];
    let mut first = 0;
    for &(path, module) in modules {
        let rewrite = rewrite(path.into(), &module.code, first)?;
        first += rewrite.regions.len();
        project.rewrite_source(&module.name, &rewrite.source)?;
        files.push(File {
            path: path.into(),
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

/// The regions that a report shows as not run, for each file of `files`.
/// A region inside another region that did not run stays out. The regions
/// come in order and do not overlap.
pub fn not_run(files: &[File]) -> Vec<(&File, Vec<&Region>)> {
    files
        .iter()
        .map(|file| {
            let mut regions: Vec<&Region> =
                file.regions.iter().filter(|region| !region.ran).collect();
            regions.sort_by_key(|region| (region.span.start, std::cmp::Reverse(region.span.end)));
            let mut shown = vec![];
            let mut end = 0;
            for region in regions {
                if region.span.start >= end {
                    shown.push(region);
                    end = region.span.end;
                }
            }
            (file, shown)
        })
        .collect()
}

/// The section of `sgleam test --coverage` that lists the regions of
/// `not_run`. Each region shows its line without the indentation, then `^`
/// under the shown span and the reason, in the colour of a warning when
/// `color` is `true`. A span that goes on to the next line is underlined up
/// to the end of the first one.
pub fn report(not_run: &[(&File, Vec<&Region>)], color: bool) -> String {
    let mut out = if color {
        Buffer::ansi()
    } else {
        Buffer::no_color()
    };
    // A write to a buffer in memory does not fail.
    let _ = write_report(&mut out, not_run);
    String::from_utf8_lossy(out.as_slice()).into_owned()
}

fn write_report(out: &mut Buffer, not_run: &[(&File, Vec<&Region>)]) -> std::io::Result<()> {
    if not_run.iter().all(|(_, regions)| regions.is_empty()) {
        return writeln!(out, "Every part of the code ran in some example.");
    }
    writeln!(out, "Code that no example ran:")?;
    let mut warning = ColorSpec::new();
    warning.set_fg(Some(Color::Yellow));
    for (file, regions) in not_run {
        let source = file.source.as_str();
        let numbers = LineNumbers::new(source);
        for region in regions {
            let span = region.shown;
            let number = numbers.line_number(span.start);
            let line_start = numbers
                .line_starts
                .get(number as usize - 1)
                .copied()
                .unwrap_or(0);
            // `lines` drops the `\r` of a file from Windows.
            let line = source[line_start as usize..]
                .lines()
                .next()
                .unwrap_or_default();
            let text = line.trim_start();
            let line_end = line_start as usize + line.len();
            write!(out, "\n  {}:{number}", file.path)?;
            // A top level function is its own name.
            if region.kind != RegionKind::Function {
                write!(out, ", in {}", region.function)?;
            }
            writeln!(out, "\n    {text}")?;
            let start = span.start as usize;
            let end = (span.end as usize).min(line_end);
            write!(
                out,
                "    {}",
                under(&source[line_end - text.len()..start], ' ')
            )?;
            let marks = under(&source[start..end], '^');
            let marks = if marks.is_empty() { "^".into() } else { marks };
            out.set_color(&warning)?;
            write!(out, "{marks} {}", reason(region.kind, source))?;
            out.reset()?;
            writeln!(out)?;
        }
    }
    Ok(())
}

/// Why no example ran a region of `kind`.
fn reason(kind: RegionKind, source: &str) -> String {
    // A left operand on many lines would not fit in the message.
    let left = |span: SrcSpan| {
        let text = &source[span.start as usize..span.end as usize];
        if text.contains('\n') {
            "the left side"
        } else {
            text
        }
    };
    match kind {
        RegionKind::Function | RegionKind::AnonymousFunction => {
            "no example calls this function".into()
        }
        RegionKind::Clause => "no example reaches this clause".into(),
        RegionKind::And { left: span } => format!("{} was never true", left(span)),
        RegionKind::Or { left: span } => format!("{} was never false", left(span)),
    }
}

/// The text that goes under `text`, with `mark` as wide as each character.
/// A tab stays a tab, so the text below lines up whatever the width of a tab
/// in the terminal.
fn under(text: &str, mark: char) -> String {
    text.chars()
        .map(|c| match c {
            '\t' => "\t".into(),
            c => mark.to_string().repeat(c.width().unwrap_or(0)),
        })
        .collect()
}

/// Returns `true` if an example never calls the function `name`, `false`
/// otherwise. Such a function would always show as not run.
fn no_example_calls(name: &str) -> bool {
    name == MainFunction::Main.name() || name == SMAIN || name.ends_with("_examples")
}

/// Rewrites `src` so that each region calls `hit`, `hit_true` or
/// `hit_false` of `sgleam/coverage` when it starts. A function that no
/// example calls, as `main`, `smain` and the examples, has no regions. The
/// numbers start at `first`, so the files of one program can share a single
/// set of numbers.
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
            if no_example_calls(&marker.function) {
                continue;
            }
            let span = SrcSpan::new(function.location.start, function.end_position);
            marker.body(
                RegionKind::Function,
                span,
                function.location,
                &function.body,
            );
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
    fn region(&mut self, kind: RegionKind, span: SrcSpan, shown: SrcSpan) -> usize {
        self.regions.push(Region {
            kind,
            span,
            shown,
            function: self.function.clone(),
            ran: false,
        });
        self.first + self.regions.len() - 1
    }

    fn insert(&mut self, at: u32, text: String) {
        self.inserts.push((at, text));
    }

    /// `head` is the part of the function before its body.
    fn body(
        &mut self,
        kind: RegionKind,
        span: SrcSpan,
        head: SrcSpan,
        statements: &[UntypedStatement],
    ) {
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
            let n = self.region(kind, span, head);
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
                FunctionLiteralKind::Anonymous { head } => {
                    self.body(RegionKind::AnonymousFunction, *location, *head, body)
                }
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
                    let left = left.location();
                    let (kind, mark, op) = match operator {
                        BinOp::And => (RegionKind::And { left }, "hit_true", "&&"),
                        _ => (RegionKind::Or { left }, "hit_false", "||"),
                    };
                    let n = self.region(kind, location, location);
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
                    let n = self.region(RegionKind::Clause, clause.location, clause.location);
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
    fn an_operand_knows_its_left_side() {
        let src = "fn f() { a && b || c }";
        let left = |kind: RegionKind| match kind {
            RegionKind::And { left } | RegionKind::Or { left } => {
                &src[left.start as usize..left.end as usize]
            }
            _ => "",
        };
        let kinds: Vec<_> = rewrite_ok(src, 0)
            .regions
            .into_iter()
            .map(|region| (region.kind.name(), left(region.kind)))
            .collect();
        assert_eq!(
            kinds,
            vec![("function", ""), ("and", "a"), ("or", "a && b")]
        );
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
                (
                    RegionKind::And {
                        left: SrcSpan::new(24, 25)
                    },
                    "b",
                    "f".into()
                ),
                (RegionKind::Function, "fn g() { fn() { 1 } }", "g".into()),
                (RegionKind::AnonymousFunction, "fn() { 1 }", "g".into()),
            ]
        );
    }

    #[test]
    fn a_function_shows_its_head() {
        let src = "pub fn f(n: Int) -> Int {\n  case n { 0 -> a && b }\n}\nfn g() { fn(x: Int) -> Int { x } }";
        let shown: Vec<_> = rewrite_ok(src, 0)
            .regions
            .into_iter()
            .map(|region| &src[region.shown.start as usize..region.shown.end as usize])
            .collect();
        assert_eq!(
            shown,
            vec![
                "pub fn f(n: Int) -> Int",
                "0 -> a && b",
                "b",
                "fn g()",
                "fn(x: Int) -> Int",
            ]
        );
    }

    fn file(path: &str, src: &str, ran: &[usize]) -> File {
        let mut regions = rewrite_ok(src, 0).regions;
        for (n, region) in regions.iter_mut().enumerate() {
            region.ran = ran.contains(&n);
        }
        File {
            path: path.into(),
            source: src.into(),
            regions,
        }
    }

    fn not_run_text(files: &[File]) -> Vec<(String, Vec<String>)> {
        not_run(files)
            .into_iter()
            .map(|(file, regions)| {
                let texts = regions
                    .iter()
                    .map(|region| {
                        let span = region.shown;
                        file.source[span.start as usize..span.end as usize].to_string()
                    })
                    .collect();
                (file.path.to_string(), texts)
            })
            .collect()
    }

    #[test]
    fn no_regions_for_what_no_example_calls() {
        let src = "pub fn main() { 1 }\npub fn smain() { 2 }\npub fn f_examples() { 3 }\npub fn f_example() { 4 }";
        assert_eq!(
            regions(src),
            vec![(
                RegionKind::Function,
                "pub fn f_example() { 4 }",
                "f_example".into()
            )]
        );
    }

    #[test]
    fn not_run_skips_a_region_inside_another_that_did_not_run() {
        let src = "fn f(n) { case n { 0 -> a && b  _ -> c } }\nfn g(n) { case n { 0 -> a && b  _ -> c } }";
        // In `g`, only the function and the first clause ran.
        assert_eq!(
            not_run_text(&[file("a.gleam", src, &[4, 5])]),
            vec![(
                "a.gleam".into(),
                vec!["fn f(n)".into(), "b".into(), "_ -> c".into()]
            )]
        );
    }

    fn report_of(src: &str, ran: &[usize], color: bool) -> String {
        let files = [file("a.gleam", src, ran)];
        report(&not_run(&files), color)
    }

    #[test]
    fn report_when_everything_ran() {
        assert_eq!(
            report_of("fn f() { 1 }", &[0], false),
            "Every part of the code ran in some example.\n"
        );
    }

    #[test]
    fn report_underlines_each_span() {
        let src = "fn f(n) {\n  case n {\n    0 -> \"ação\" == n && b\n    _ -> c\n  }\n}\n";
        assert_eq!(
            report_of(src, &[0, 1], false),
            "Code that no example ran:\n\n  \
             a.gleam:3, in f\n    0 -> \"ação\" == n && b\n                        \
             ^ \"ação\" == n was never true\n\n  \
             a.gleam:4, in f\n    _ -> c\n    ^^^^^^ no example reaches this clause\n"
        );
    }

    #[test]
    fn report_has_an_entry_for_each_span_of_a_line() {
        let src = "fn f() { a && b || c }";
        assert_eq!(
            report_of(src, &[0], false),
            "Code that no example ran:\n\n  \
             a.gleam:1, in f\n    fn f() { a && b || c }\n                  ^ a was never true\n\n  \
             a.gleam:1, in f\n    fn f() { a && b || c }\n                       \
             ^ a && b was never false\n"
        );
    }

    #[test]
    fn report_underlines_a_long_span_to_the_end_of_its_line() {
        let src = "fn f(n) {\r\n\tcase n {\r\n\t\t0 -> a\t&& {\r\n\t\t\tb\r\n\t\t}\r\n\t}\r\n}\r\n";
        assert_eq!(
            report_of(src, &[0, 1], false),
            "Code that no example ran:\n\n  \
             a.gleam:3, in f\n    0 -> a\t&& {\n          \t   ^ a was never true\n"
        );
    }

    #[test]
    fn report_keeps_a_tab_inside_the_span() {
        let src = "fn f(n) { case n { 0 -> a\t&&\tb } }";
        assert_eq!(
            report_of(src, &[0], false),
            "Code that no example ran:\n\n  \
             a.gleam:1, in f\n    fn f(n) { case n { 0 -> a\t&&\tb } }\n                       \
             ^^^^^^\t^^\t^ no example reaches this clause\n"
        );
    }

    #[test]
    fn report_names_the_function_of_an_anonymous_one() {
        assert_eq!(
            report_of("fn g() { fn(x) { x } }", &[0], false),
            "Code that no example ran:\n\n  \
             a.gleam:1, in g\n    fn g() { fn(x) { x } }\n             ^^^^^ no example calls this function\n"
        );
    }

    #[test]
    fn report_says_the_left_side_when_it_has_many_lines() {
        let src = "fn f() {\n  g(\n    a,\n  ) && b\n}";
        assert_eq!(
            report_of(src, &[0], false),
            "Code that no example ran:\n\n  \
             a.gleam:4, in f\n    ) && b\n         ^ the left side was never true\n"
        );
    }

    #[test]
    fn report_paints_the_underline() {
        let src = "fn f() { 1 }";
        assert_eq!(
            report_of(src, &[], true),
            "Code that no example ran:\n\n  a.gleam:1\n    fn f() { 1 }\n    \
             \u{1b}[0m\u{1b}[33m^^^^^^ no example calls this function\u{1b}[0m\n"
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

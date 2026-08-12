//! The source of a module the repl writes, with the origin of every byte kept.

use std::{fmt, rc::Rc};

use gleam_core::{ast::SrcSpan, line_numbers::LineNumbers};

/// A module the repl writes: what it says itself, and the regions it copied
/// from the input. Moving a diagnostic back onto the input is then a lookup
/// instead of a search through the text for something that looks like it.
///
/// A copy carries where in the input it came from, so the repl can write
/// between two halves of one definition — which is how a body gets the
/// bindings it reads — and the halves still read against the whole input, at
/// the line and column the user sees.
///
/// `fmt::Write` writes the repl's own text, so `swriteln!` says the same thing
/// as [`Source::write`].
#[derive(Clone, Default)]
pub struct Source {
    text: String,
    /// Disjoint, and in increasing order, by construction.
    copies: Vec<Copied>,
}

#[derive(Clone)]
struct Copied {
    /// Where the copy starts in the generated text.
    at: u32,
    /// The input, which is what a diagnostic moved onto the copy is read
    /// against, and where in it the copy was taken.
    input: Rc<str>,
    from: u32,
    len: u32,
}

/// What a span of the generated text points at in the input.
pub struct Located<'a> {
    pub input: &'a Rc<str>,
    pub span: SrcSpan,
}

impl Source {
    pub fn new() -> Source {
        Source::default()
    }

    /// Writes what the repl is saying itself.
    pub fn write(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Writes `input[span]`, which is where a diagnostic about it lands.
    pub fn copy(&mut self, input: &Rc<str>, span: SrcSpan) {
        // Nothing copied is nothing to point at.
        if span.start == span.end {
            return;
        }
        let (start, end) = (span.start as usize, span.end as usize);
        self.copies.push(Copied {
            at: self.text.len() as u32,
            input: input.clone(),
            from: span.start,
            len: span.end - span.start,
        });
        self.text.push_str(&input[start..end]);
    }

    /// Appends `other`, keeping the origin of what it carries.
    pub fn append(&mut self, other: &Source) {
        let at = self.text.len() as u32;
        self.copies.extend(other.copies.iter().map(|copy| Copied {
            at: copy.at + at,
            ..copy.clone()
        }));
        self.text.push_str(&other.text);
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// What `span` points at in the input: the smallest range holding every
    /// byte of it the span takes in. `None` when it takes in none — the repl
    /// wrote that text, and the user cannot be shown a place they did not
    /// write.
    ///
    /// A span that reaches over what the repl put in — the `pub` before a
    /// definition, the bindings at the top of a body — is about the definition
    /// it is written into, which is the user's. Narrowing it to their bytes is
    /// what says so.
    pub fn locate(&self, span: SrcSpan) -> Option<Located<'_>> {
        let mut located: Option<Located> = None;
        for copy in &self.copies {
            let (start, end) = (span.start.max(copy.at), span.end.min(copy.at + copy.len));
            // A span of no width still points somewhere; one that has width has
            // to take a byte in, not merely touch an edge.
            let takes_in = if span.start == span.end {
                start <= end
            } else {
                start < end
            };
            if !takes_in {
                continue;
            }
            let taken = SrcSpan::new(copy.from + start - copy.at, copy.from + end - copy.at);
            match &mut located {
                // A diagnostic is read against one input, so a copy of another
                // says nothing about where this one is.
                Some(located) if !Rc::ptr_eq(located.input, &copy.input) => {}
                Some(located) => {
                    located.span.start = located.span.start.min(taken.start);
                    located.span.end = located.span.end.max(taken.end);
                }
                None => {
                    located = Some(Located {
                        input: &copy.input,
                        span: taken,
                    })
                }
            }
        }
        located
    }

    /// The line of the input each line of the generated text was copied from,
    /// indexed by line — so the first element stands for no line at all, and 0
    /// for a line the repl wrote itself.
    ///
    /// This is what names a place in a generated module by the input it came
    /// from, for a runtime that has only the line to go on: `echo` is compiled
    /// to the file and line it was written at, and the file is one the user
    /// never saw.
    pub fn input_lines(&self) -> Vec<u32> {
        let mut lines = vec![0];
        let mut at = 0;
        for line in self.text.split_inclusive('\n') {
            let end = at + line.len() as u32;
            lines.push(match self.locate(SrcSpan::new(at, end)) {
                Some(located) => LineNumbers::new(located.input).line_number(located.span.start),
                None => 0,
            });
            at = end;
        }
        lines
    }
}

impl fmt::Write for Source {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.write(text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u32, end: u32) -> SrcSpan {
        SrcSpan { start, end }
    }

    /// A whole input of one line.
    fn copy_all(src: &mut Source, text: &str) {
        src.copy(&text.into(), span(0, text.len() as u32));
    }

    #[test]
    fn a_span_over_what_the_repl_wrote_is_not_the_input() {
        let mut src = Source::new();
        src.write("pub fn main() {\n");
        copy_all(&mut src, "1 + 1");
        src.write("\n}\n");

        assert_eq!(src.as_str(), "pub fn main() {\n1 + 1\n}\n");
        assert!(src.locate(span(0, 3)).is_none());
        assert!(src.locate(span(21, 24)).is_none());
        assert_eq!(src.locate(span(16, 21)).unwrap().span, span(0, 5));
        assert_eq!(src.locate(span(20, 21)).unwrap().span, span(4, 5));
        // No width, so it points at a place instead of taking bytes in.
        assert_eq!(src.locate(span(21, 21)).unwrap().span, span(5, 5));
    }

    /// The repl writes `pub` in front of a definition and its bindings into a
    /// body, so a span over a whole definition is a span over what both wrote.
    /// It is the user's definition, and pointing at their bytes says so.
    #[test]
    fn a_span_over_both_narrows_to_what_the_user_wrote() {
        let input: Rc<str> = "fn f() { 1 }".into();
        let mut src = Source::new();
        src.write("pub ");
        src.copy(&input, span(0, 8));
        src.write("let x = x()\n");
        src.copy(&input, span(8, 12));

        assert_eq!(src.as_str(), "pub fn f() {let x = x()\n 1 }");
        // The head, as the compiler reports it: `pub fn f()`.
        assert_eq!(src.locate(span(0, 10)).unwrap().span, span(0, 6));
        // The definition whole, over both halves and what went between them.
        assert_eq!(src.locate(span(0, 28)).unwrap().span, span(0, 12));
    }

    /// The same text twice, only one of which is the copy the input is read
    /// against — which is what searching for it cannot tell apart.
    #[test]
    fn the_copy_is_found_by_where_it_is_not_by_what_it_says() {
        let mut src = Source::new();
        src.write("let x = 1\n");
        copy_all(&mut src, "let x = 1");

        assert!(src.locate(span(0, 9)).is_none());
        assert!(src.locate(span(10, 19)).is_some());
    }

    #[test]
    fn the_input_line_of_every_generated_line() {
        let input: Rc<str> = "fn f(x) {\n  echo x\n}".into();
        let mut src = Source::new();
        src.write("import gleam/io\npub ");
        src.copy(&input, span(0, 10));
        src.write("let a = a()\n");
        src.copy(&input, span(10, 20));

        assert_eq!(
            src.as_str(),
            "import gleam/io\npub fn f(x) {\nlet a = a()\n  echo x\n}"
        );
        // Index by line: 0 stands for no line, and a line the repl wrote for
        // itself came from none of the input.
        assert_eq!(src.input_lines(), vec![0, 0, 1, 0, 2, 3]);
    }

    #[test]
    fn appending_moves_the_copies_it_carries() {
        let mut body = Source::new();
        body.write("echo ");
        copy_all(&mut body, "x");

        let mut src = Source::new();
        src.write("import gleam/io\n");
        src.append(&body);

        assert_eq!(src.as_str(), "import gleam/io\necho x");
        assert_eq!(src.locate(span(21, 22)).unwrap().input.as_ref(), "x");
    }

    /// What the repl writes between two halves of one definition, which is how
    /// a body gets the bindings it reads: both halves still read against the
    /// input, at the position the user wrote them in.
    #[test]
    fn a_definition_split_around_what_the_repl_writes() {
        let input: Rc<str> = "fn f(a) {\n  a + x\n}".into();
        let mut src = Source::new();
        src.copy(&input, span(0, 10));
        src.write("let x = x()\n");
        src.copy(&input, span(10, 19));

        assert_eq!(src.as_str(), "fn f(a) {\nlet x = x()\n  a + x\n}");
        // `x` of the second half, which the input has at 16.
        assert_eq!(src.locate(span(28, 29)).unwrap().span, span(16, 17));
        // The `x` the repl wrote is not the input's.
        assert!(src.locate(span(14, 15)).is_none());
    }

    #[test]
    fn several_copies_of_one_input() {
        let input: Rc<str> = "fn f() { 1 } fn g() { 2 }".into();
        let mut src = Source::new();
        src.write("pub ");
        src.copy(&input, span(0, 12));
        src.write("\npub ");
        src.copy(&input, span(13, 25));

        assert_eq!(src.locate(span(4, 16)).unwrap().span, span(0, 12));
        assert_eq!(src.locate(span(21, 33)).unwrap().span, span(13, 25));
    }
}

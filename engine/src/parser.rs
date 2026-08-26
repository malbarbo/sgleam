use gleam_core::{
    ast::{TargetedDefinition, UntypedStatement},
    parse::{
        Parser,
        error::{LexicalError, LexicalErrorType, ParseError, ParseErrorType},
        lexer::{self, LexResult},
        token::Token,
    },
};

#[derive(Debug)]
pub enum ReplItem {
    /// A definition and where the input starts it: the location the parser
    /// records begins at the keyword, leaving the attributes above it out.
    ReplDefinition(TargetedDefinition, u32),
    ReplStatement(UntypedStatement),
}

pub fn parse_repl(src: &str) -> Result<Vec<ReplItem>, ParseError> {
    let lex = lexer::make_tokenizer(src);
    let mut parser = Parser::new(lex);
    let definitions = parser.series_of(&Parser::parse_definition_or_statement, None);
    parser.ensure_no_errors_or_remaining_input(definitions)
}

/// Whether the input ends before what it started does, which the reader cannot
/// see in the text: a bracket inside a comment closes nothing, a string runs to
/// the next line, and `use a <-` is unfinished with nothing open at all.
///
/// The kind of error does not say it. `let x =` and `let x = )` are the same
/// `ExpectedValue` over the same `=`, as the parser reports where it asked for
/// a value and not what it found instead. What tells them apart is whether any
/// of the input is left after that: if none is, there was nothing to reject.
///
/// A wrong `true` hangs the prompt on a stray `}`, so an error that says
/// neither is taken as finished.
pub fn is_incomplete(src: &str) -> bool {
    let Err(ParseError { error, location }) = parse_repl(src) else {
        return false;
    };
    match error {
        // No token was left to reject.
        ParseErrorType::UnexpectedEof
        | ParseErrorType::LexError {
            error:
                LexicalError {
                    error: LexicalErrorType::UnexpectedStringEnd,
                    ..
                },
        } => true,
        ParseErrorType::ExpectedEqual
        | ParseErrorType::ExpectedExpr
        | ParseErrorType::ExpectedFunctionBody
        | ParseErrorType::ExpectedType
        | ParseErrorType::ExpectedValue
        | ParseErrorType::NoValueAfterEqual
        | ParseErrorType::OpNakedRight
        // Reported at the attributes, above what they attach to.
        | ParseErrorType::ExpectedDefinition
        | ParseErrorType::ExpectedFunctionDefinition => ends_there(src, location.end),
        // The one token that opens a definition and says nothing else.
        ParseErrorType::UnexpectedToken {
            token: Token::Pub, ..
        } => ends_there(src, location.end),
        _ => false,
    }
}

/// Whether nothing of the input comes after `at`. A comment and a blank line
/// are not something: they are what the user typed before going on.
fn ends_there(src: &str, at: u32) -> bool {
    lexer::make_tokenizer(src)
        .flatten()
        .filter(|(_, token, _)| {
            !matches!(
                token,
                Token::NewLine
                    | Token::CommentNormal
                    | Token::CommentModule
                    | Token::CommentDoc { .. }
            )
        })
        .all(|(start, _, _)| start < at)
}

/// How deep in brackets the input ends, which is what the next line is
/// indented by. Counted in tokens, so a bracket inside a comment or a string
/// is text.
///
/// Every kind of bracket is worth a level: a call or a list left open asks for
/// the next line to be indented as much as a block does. This counts what is
/// still open and nothing else, so a block hugging the call that opens it --
/// `list.map(l, fn(x) {` -- is two levels here where the formatter writes one.
pub fn nesting_depth(src: &str) -> usize {
    let mut depth: i32 = 0;
    for token in lexer::make_tokenizer(src).flatten() {
        match token.1 {
            Token::LeftBrace | Token::LeftParen | Token::LeftSquare | Token::LtLt => depth += 1,
            Token::RightBrace | Token::RightParen | Token::RightSquare | Token::GtGt => depth -= 1,
            _ => {}
        }
    }
    depth.max(0) as usize
}

trait ParserRepl {
    fn parse_definition_or_statement(parser: &mut Self) -> Result<Option<ReplItem>, ParseError>;
}

impl<T> ParserRepl for Parser<T>
where
    T: Iterator<Item = LexResult>,
{
    fn parse_definition_or_statement(parser: &mut Self) -> Result<Option<ReplItem>, ParseError> {
        let (tok0, tok1) = parser.tok01();
        // special case for anonymous function
        if let (Some((_, Token::Fn, _)), Some((_, Token::LeftParen, _))) = (&tok0, &tok1) {
            return Ok(parser.parse_statement()?.map(ReplItem::ReplStatement));
        }
        let start = tok0.map(|(start, _, _)| start).unwrap_or_default();
        if let Some(def) = parser.parse_definition()? {
            return Ok(Some(ReplItem::ReplDefinition(def, start)));
        }
        if let Some(sta) = parser.parse_statement()? {
            return Ok(Some(ReplItem::ReplStatement(sta)));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_input_that_stops_where_it_asked_for_more() {
        for src in [
            "let x =",
            "let x = 1 +",
            "1 +",
            "x |>",
            "let x = \"a\" <>",
            "const x =",
            "let x: ",
            "fn f() -> ",
            "case x { 1 ->",
            "let assert Ok(x) =",
            "let x = [1,",
            "case x {",
            "let x = \"abc",
            "use a <-",
            "import gleam/",
            "@deprecated(\"old\")",
            "@target(javascript)",
            "@external(javascript, \"./x.mjs\", \"f\")",
            "pub",
            "let x = 1 + // hi",
            "let x =\n\n",
            "case x {\n  1 -> \n",
        ] {
            assert!(is_incomplete(src), "{src:?}");
        }
    }

    #[test]
    fn an_input_that_is_finished_and_wrong() {
        for src in [
            "let = 1",
            "let x = )",
            "let x = ) 1",
            "let x = 1 +)",
            "1 + + 1",
            "x |> |> y",
            "const x = = 1",
            "let x: = 1",
            "fn f() -> { 1 }",
            "fn 1() {}",
            "case x { 1 -> }",
            "case x { -> 1 }",
            "let #(a, = 1",
            "@deprecated(\"old\") 1",
            "@external(javascript, \"./x.mjs\", \"f\") type T { T }",
            "pub 1",
            // A closer with nothing open, which is the one that must not hang.
            "}",
            ")",
            "let x = 1 } ",
            "fn f() { 1 } )",
            "import gleam/list }",
            "let x = 1",
            "1 + 1",
            "fn f() { 1 }",
            "let x = 1 1",
            "// comment",
        ] {
            assert!(!is_incomplete(src), "{src:?}");
        }
    }

    #[test]
    fn how_deep_in_brackets_the_input_ends() {
        assert_eq!(nesting_depth("let x = 1"), 0);
        assert_eq!(nesting_depth("case x {"), 1);
        assert_eq!(nesting_depth("list.map(l, fn(x) {"), 2);
        assert_eq!(nesting_depth("let x = <<1,"), 1);
        assert_eq!(nesting_depth("let x = <<1, 2>>"), 0);
        // A bracket the parser never reads is text.
        assert_eq!(nesting_depth("let x = [1, // ]\n"), 1);
        assert_eq!(nesting_depth("let x = \"(\""), 0);
        // More closers than openers is still the outermost level.
        assert_eq!(nesting_depth("}"), 0);
    }

    /// Every prefix of a valid input is unfinished, by construction. Not all of
    /// them are read as such — an error the parser reports away from where the
    /// input stopped says nothing — so this holds the count it reaches.
    #[test]
    fn most_of_a_valid_input_read_as_unfinished() {
        let src = r#"import gleam/int

pub type Shape {
  Circle(r: Float)
  Square(side: Float)
}

pub fn area(shape: Shape) -> Float {
  case shape {
    Circle(r) -> 3.14 *. r *. r
    Square(s) -> s *. s
  }
}

pub fn main() {
  let shapes = [Circle(1.0), Square(2.0)]
  let total = shapes |> list.map(area) |> float.sum
  io.println(int.to_string(total) <> " units")
}
"#;
        let ends: Vec<u32> = lexer::make_tokenizer(src)
            .flatten()
            .map(|(_, _, end)| end)
            .collect();
        let (mut unfinished, mut read) = (0, 0);
        for end in ends {
            let prefix = &src[..end as usize];
            if parse_repl(prefix).is_err() {
                unfinished += 1;
                read += usize::from(is_incomplete(prefix));
            }
        }
        assert!(unfinished > 100, "{unfinished} prefixes is too few to say");
        assert!(
            read * 100 / unfinished >= 95,
            "{read} of {unfinished} prefixes read as unfinished"
        );
    }
}

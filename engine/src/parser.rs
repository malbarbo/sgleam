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
    /// A definition and where the input starts it. The location the parser
    /// records begins at the keyword, so the attributes above it are only
    /// reachable from the token the item opened with.
    ReplDefinition(TargetedDefinition, u32),
    ReplStatement(UntypedStatement),
}

pub fn parse_repl(src: &str) -> Result<Vec<ReplItem>, ParseError> {
    let lex = lexer::make_tokenizer(src);
    let mut parser = Parser::new(lex);
    let definitions = parser.series_of(&Parser::parse_definition_or_statement, None);
    parser.ensure_no_errors_or_remaining_input(definitions)
}

/// Whether the input ends before what it started does, which is the one thing
/// a reader has to know and cannot see in the text: a bracket inside a comment
/// closes nothing, a string runs to the next line, and `use a <-` is unfinished
/// with nothing open at all.
///
/// Only these two say so. Everything else the parser rejects is finished and
/// wrong — `let x =` is a typo, not a line to go on typing — and a prompt that
/// waited for it would have no way out.
pub fn is_incomplete(src: &str) -> bool {
    matches!(
        parse_repl(src),
        Err(ParseError {
            error: ParseErrorType::UnexpectedEof
                | ParseErrorType::LexError {
                    error: LexicalError {
                        error: LexicalErrorType::UnexpectedStringEnd,
                        ..
                    }
                },
            ..
        })
    )
}

/// How deep in blocks the input ends, which is what the next line is indented
/// by. Counted in tokens, so a brace inside a comment or a string is text and
/// not a block.
pub fn nesting_depth(src: &str) -> usize {
    let mut depth: i32 = 0;
    for token in lexer::make_tokenizer(src).flatten() {
        match token.1 {
            Token::LeftBrace => depth += 1,
            Token::RightBrace => depth -= 1,
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

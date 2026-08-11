use gleam_core::{
    ast::{TargetedDefinition, UntypedStatement},
    parse::{
        Parser,
        error::ParseError,
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

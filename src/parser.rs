use gleam_core::{
    ast::{TargetedDefinition, UntypedStatement},
    parse::{
        error::ParseError,
        lexer::{self, LexResult},
        token::Token,
        Parser,
    },
};

#[derive(Debug)]
pub enum ReplItem {
    ReplDefinition(TargetedDefinition),
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
        // special case for anonymous function
        if let (Some((_, Token::Fn, _)), Some((_, Token::LeftParen, _))) = parser.tok01() {
            return Ok(parser.parse_statement()?.map(ReplItem::ReplStatement));
        }
        if let Some(def) = parser.parse_definition()? {
            return Ok(Some(ReplItem::ReplDefinition(def)));
        }
        if let Some(sta) = parser.parse_statement()? {
            return Ok(Some(ReplItem::ReplStatement(sta)));
        }
        return Ok(None);
    }
}

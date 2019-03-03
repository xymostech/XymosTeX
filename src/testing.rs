use crate::category::Category;
use crate::parser::Parser;
use crate::state::TeXState;
use crate::token::Token;

#[cfg(test)]
pub fn with_parser<T>(lines: &[&str], cb: T)
where
    T: FnOnce(&mut Parser),
{
    let state = TeXState::new();
    let mut parser = Parser::new(lines, &state);

    cb(&mut parser);
    assert_eq!(parser.lex_unexpanded_token(), None);
}

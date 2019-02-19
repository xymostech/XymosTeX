use crate::lexer::Lexer;
use crate::state::TeXState;
use crate::token::Token;

#[allow(dead_code)] // TODO(xymostech): remove this once state is used
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    state: &'a TeXState,

    upcoming_tokens: Vec<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(lines: &[&str], state: &'a TeXState) -> Parser<'a> {
        let lexer = Lexer::new(lines, &state);
        Parser {
            lexer: lexer,
            state: state,
            upcoming_tokens: Vec::new(),
        }
    }
}

mod assignment;
mod expand;
mod horizontal_list;
mod makro;

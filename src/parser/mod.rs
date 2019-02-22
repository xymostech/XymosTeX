use crate::lexer::Lexer;
use crate::state::TeXState;
use crate::token::Token;

#[allow(dead_code)] // TODO(xymostech): remove this once state is used
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    state: &'a TeXState,

    // Used in expand module to keep track of the next tokens to parse
    upcoming_tokens: Vec<Token>,

    // Used in conditional module to keep track of the level of nesting of
    // conditionals
    conditional_depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(lines: &[&str], state: &'a TeXState) -> Parser<'a> {
        let lexer = Lexer::new(lines, &state);
        Parser {
            lexer: lexer,
            state: state,
            upcoming_tokens: Vec::new(),
            conditional_depth: 0,
        }
    }
}

mod assignment;
mod conditional;
mod expand;
mod horizontal_list;
mod makro;
mod primitives;

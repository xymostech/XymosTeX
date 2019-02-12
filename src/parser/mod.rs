use crate::lexer::Lexer;
use crate::state::TeXState;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    state: &'a TeXState,
}

impl<'a> Parser<'a> {
    pub fn new(lines: &[&str], state: &'a TeXState) -> Parser<'a> {
        let lexer = Lexer::new(lines, &state);
        Parser {
            lexer: lexer,
            state: state,
        }
    }
}

mod horizontal_list;

use crate::lexer::Lexer;
use crate::state::TeXState;
use crate::token::Token;

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
    pub fn new<T>(lines: &[T], state: &'a TeXState) -> Parser<'a>
    where
        T: AsRef<str>,
        T: std::string::ToString,
    {
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
mod boxes;
mod conditional;
mod dimen;
mod expand;
mod glue;
mod horizontal_list;
mod makro;
mod number;
mod primitives;
mod printing;
mod variable;
mod vertical_list;

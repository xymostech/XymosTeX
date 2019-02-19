use std::rc::Rc;

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_macro_assignment_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => token == Token::ControlSequence("def".to_string()),
            None => false,
        }
    }

    pub fn is_assignment_head(&mut self) -> bool {
        self.is_macro_assignment_head()
    }

    // Parses a control sequence or special char token to use for \def or \let
    // names
    fn parse_unexpanded_control_sequence(&mut self) -> Token {
        match self.lex_unexpanded_token() {
            Some(token) => match token {
                Token::ControlSequence(_) => token,
                Token::Char(_, Category::Active) => token,
                _ => panic!(
                    "Invalid token found while looking for control sequence: {:?}",
                    token
                ),
            },
            None => panic!("EOF found parsing control sequence"),
        }
    }

    fn parse_macro_assignment(&mut self) {
        let tok = self.lex_expanded_token().unwrap();

        if tok == Token::ControlSequence("def".to_string()) {
            let control_sequence = self.parse_unexpanded_control_sequence();
            let makro = self.parse_macro_definition();

            self.state.set_macro(control_sequence, Rc::new(makro));
        }
    }

    pub fn parse_assignment(&mut self) {
        if self.is_macro_assignment_head() {
            self.parse_macro_assignment()
        } else {
            panic!("Non-macro head found in parse_assignment");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::category::Category;
    use crate::makro::{Macro, MacroListElem};
    use crate::state::TeXState;

    #[test]
    fn it_assigns_macros() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\def\\a #1x{#1y#1}%"], &state);

        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            *state
                .get_macro(&Token::ControlSequence("a".to_string()))
                .unwrap(),
            Macro::new(
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                ],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('y', Category::Letter)),
                    MacroListElem::Parameter(1),
                ]
            )
        );
    }
}

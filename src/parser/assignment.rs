use std::rc::Rc;

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_macro_assignment_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(Token::ControlSequence(cs)) => cs == "def",
            _ => false,
        }
    }

    fn is_let_assignment_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(Token::ControlSequence(cs)) => cs == "let",
            _ => false,
        }
    }

    pub fn is_assignment_head(&mut self) -> bool {
        self.is_macro_assignment_head() || self.is_let_assignment_head()
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

    fn parse_let_assignment(&mut self) {
        let tok = self.lex_expanded_token().unwrap();

        if tok == Token::ControlSequence("let".to_string()) {
            let let_name = self.parse_unexpanded_control_sequence();
            self.parse_equals_unexpanded();
            self.parse_optional_space_unexpanded();
            let let_value = self.lex_unexpanded_token().unwrap();

            self.state.set_let(false, &let_name, &let_value);
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_macro_assignment(&mut self) {
        let tok = self.lex_expanded_token().unwrap();

        if tok == Token::ControlSequence("def".to_string()) {
            let control_sequence = self.parse_unexpanded_control_sequence();
            let makro = self.parse_macro_definition();

            self.state
                .set_macro(false, &control_sequence, &Rc::new(makro));
        }
    }

    pub fn parse_assignment(&mut self) {
        if self.is_macro_assignment_head() {
            self.parse_macro_assignment()
        } else if self.is_let_assignment_head() {
            self.parse_let_assignment()
        } else {
            panic!("Invalid start found in parse_assignment");
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

    #[test]
    fn it_assigns_lets_for_characters() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\let\\a=b%"], &state);

        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            state.get_renamed_token(&Token::ControlSequence("a".to_string())),
            Some(Token::Char('b', Category::Letter))
        );
    }

    #[test]
    fn it_assigns_lets_for_previously_defined_macros() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\def\\a{x}%", "\\let\\b=\\a%"], &state);

        parser.parse_assignment();
        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            *state
                .get_macro(&Token::ControlSequence("b".to_string()))
                .unwrap(),
            Macro::new(
                vec![],
                vec![MacroListElem::Token(Token::Char('x', Category::Letter)),]
            )
        );
    }

    #[test]
    fn it_doesnt_assign_lets_for_active_tokens() {
        let state = TeXState::new();
        state.set_category(false, '@', Category::Active);
        let mut parser = Parser::new(&["\\let\\a=@%"], &state);

        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            state.get_renamed_token(&Token::ControlSequence("a".to_string())),
            None
        );
    }

    #[test]
    fn it_assigns_lets_for_primitives() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\let\\a=\\def%"], &state);

        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert!(state.is_token_equal_to_cs(&Token::ControlSequence("a".to_string()), "def"));
    }
}

use std::rc::Rc;

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_variable_assignment_head(&mut self) -> bool {
        self.is_variable_head()
    }

    fn is_macro_assignment_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => self.state.is_token_equal_to_cs(&token, "def"),
            _ => false,
        }
    }

    fn is_let_assignment_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => self.state.is_token_equal_to_cs(&token, "let"),
            _ => false,
        }
    }

    fn is_non_macro_assignment_head(&mut self) -> bool {
        self.is_let_assignment_head() || self.is_variable_assignment_head()
    }

    pub fn is_assignment_head(&mut self) -> bool {
        self.is_macro_assignment_head() || self.is_non_macro_assignment_head()
    }

    fn parse_variable_assignment(&mut self, global: bool) {
        let variable = self.parse_variable();
        self.parse_equals_expanded();
        let value = self.parse_number_value();
        variable.set(self.state, global, value);
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

    fn parse_let_assignment(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_cs(&tok, "let") {
            let let_name = self.parse_unexpanded_control_sequence();
            self.parse_equals_unexpanded();
            self.parse_optional_space_unexpanded();
            let let_value = self.lex_unexpanded_token().unwrap();

            self.state.set_let(global, &let_name, &let_value);
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_macro_assignment(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_cs(&tok, "def") {
            let control_sequence = self.parse_unexpanded_control_sequence();
            let makro = self.parse_macro_definition();

            self.state
                .set_macro(global, &control_sequence, &Rc::new(makro));
        }
    }

    fn parse_non_macro_assignment(&mut self, global: bool) {
        if self.is_variable_assignment_head() {
            self.parse_variable_assignment(global)
        } else if self.is_let_assignment_head() {
            self.parse_let_assignment(global)
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_assignment_global(&mut self, global: bool) {
        if self.is_macro_assignment_head() {
            self.parse_macro_assignment(global)
        } else if self.is_non_macro_assignment_head() {
            self.parse_non_macro_assignment(global)
        } else {
            let tok = self.lex_expanded_token().unwrap();
            if self.state.is_token_equal_to_cs(&tok, "global") {
                if self.is_assignment_head() {
                    self.parse_assignment_global(true);
                } else {
                    panic!("Non-assignment head found after \\global");
                }
            } else {
                panic!("Invalid start found in parse_assignment");
            }
        }
    }

    pub fn parse_assignment(&mut self) {
        self.parse_assignment_global(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::category::Category;
    use crate::makro::{Macro, MacroListElem};
    use crate::state::TeXState;
    use crate::testing::with_parser;

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
    fn it_sets_global_defs() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\global\\def\\a{x}%"], &state);

        state.push_state();
        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);
        state.pop_state();

        assert_eq!(
            *state
                .get_macro(&Token::ControlSequence("a".to_string()))
                .unwrap(),
            Macro::new(
                vec![],
                vec![MacroListElem::Token(Token::Char('x', Category::Letter)),]
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
        let mut parser =
            Parser::new(&["\\def\\a{x}%", "\\let\\b=\\a%"], &state);

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

        assert!(state.is_token_equal_to_cs(
            &Token::ControlSequence("a".to_string()),
            "def"
        ));
    }

    #[test]
    fn it_lets_let_be_let() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\let\\a=\\let%", "\\a\\x=y%"], &state);

        parser.parse_assignment();
        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            state.get_renamed_token(&Token::ControlSequence("x".to_string())),
            Some(Token::Char('y', Category::Letter))
        );
    }

    #[test]
    fn it_lets_def_be_let() {
        let state = TeXState::new();
        let mut parser =
            Parser::new(&["\\let\\a=\\def%", "\\a\\x #1{#1}%"], &state);

        parser.parse_assignment();
        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);

        assert_eq!(
            *state
                .get_macro(&Token::ControlSequence("x".to_string()))
                .unwrap(),
            Macro::new(
                vec![MacroListElem::Parameter(1),],
                vec![MacroListElem::Parameter(1),]
            )
        );
    }

    #[test]
    fn it_sets_global_lets() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\global\\let\\a=b%"], &state);

        state.push_state();
        parser.parse_assignment();
        assert_eq!(parser.lex_unexpanded_token(), None);
        state.pop_state();

        assert_eq!(
            state.get_renamed_token(&Token::ControlSequence("a".to_string())),
            Some(Token::Char('b', Category::Letter))
        );
    }

    #[test]
    fn it_sets_count_variables() {
        with_parser(
            &["\\count0=2%", "\\count100 -12345%", "\\count10=\\count100%"],
            |parser| {
                parser.parse_assignment();
                parser.parse_assignment();
                parser.parse_assignment();

                assert_eq!(parser.state.get_count(0), 2);
                assert_eq!(parser.state.get_count(100), -12345);
                assert_eq!(parser.state.get_count(10), -12345);
            },
        );
    }

    #[test]
    fn it_sets_count_variables_globally() {
        with_parser(&["\\global\\count0=2%"], |parser| {
            parser.state.push_state();
            parser.parse_assignment();
            parser.state.pop_state();

            assert_eq!(parser.state.get_count(0), 2);
        });
    }
}

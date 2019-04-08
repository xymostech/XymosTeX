use std::rc::Rc;

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_variable_assignment_head(&mut self) -> bool {
        self.is_variable_head()
    }

    fn is_macro_assignment_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["def"])
    }

    fn is_let_assignment_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["let"])
    }

    fn is_arithmetic_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "advance", "multiply", "divide",
        ])
    }

    fn is_simple_assignment_head(&mut self) -> bool {
        self.is_let_assignment_head()
            || self.is_variable_assignment_head()
            || self.is_arithmetic_head()
    }

    fn is_assignment_prefix(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["global"])
    }

    pub fn is_assignment_head(&mut self) -> bool {
        self.is_assignment_prefix()
            || self.is_macro_assignment_head()
            || self.is_simple_assignment_head()
    }

    fn parse_variable_assignment(&mut self, global: bool) {
        let variable = self.parse_variable();
        self.parse_equals_expanded();
        let value = self.parse_number();
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

        if self.state.is_token_equal_to_prim(&tok, "let") {
            let let_name = self.parse_unexpanded_control_sequence();
            self.parse_equals_unexpanded();
            self.parse_optional_space_unexpanded();
            let let_value = self.lex_unexpanded_token().unwrap();

            self.state.set_let(global, &let_name, &let_value);
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_arithmetic(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();
        let variable = self.parse_variable();
        self.parse_optional_keyword_expanded("by");
        self.parse_optional_spaces_expanded();

        if self.state.is_token_equal_to_prim(&tok, "advance") {
            let number = self.parse_number();
            // TODO(xymostech): ensure this doesn't overflow
            variable.set(self.state, global, variable.get(self.state) + number);
        } else if self.state.is_token_equal_to_prim(&tok, "multiply") {
            let number = self.parse_number();
            // TODO(xymostech): ensure this doesn't overflow
            variable.set(self.state, global, variable.get(self.state) * number);
        } else if self.state.is_token_equal_to_prim(&tok, "divide") {
            let number = self.parse_number();
            variable.set(self.state, global, variable.get(self.state) / number);
        } else {
            panic!("Invalid arithmetic head: {:?}", tok);
        }
    }

    fn parse_macro_assignment(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&tok, "def") {
            let control_sequence = self.parse_unexpanded_control_sequence();
            let makro = self.parse_macro_definition();

            self.state
                .set_macro(global, &control_sequence, &Rc::new(makro));
        }
    }

    fn parse_simple_assignment(&mut self, global: bool) {
        if self.is_variable_assignment_head() {
            self.parse_variable_assignment(global)
        } else if self.is_let_assignment_head() {
            self.parse_let_assignment(global)
        } else if self.is_arithmetic_head() {
            self.parse_arithmetic(global)
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_assignment_global(&mut self, global: bool) {
        if self.is_macro_assignment_head() {
            self.parse_macro_assignment(global)
        } else if self.is_simple_assignment_head() {
            self.parse_simple_assignment(global)
        } else {
            let tok = self.lex_expanded_token().unwrap();
            if self.state.is_token_equal_to_prim(&tok, "global") {
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
    use crate::testing::with_parser;

    #[test]
    fn it_assigns_macros() {
        with_parser(&["\\def\\a #1x{#1y#1}%"], |parser| {
            parser.parse_assignment();

            assert_eq!(
                *parser
                    .state
                    .get_macro(&Token::ControlSequence("a".to_string()))
                    .unwrap(),
                Macro::new(
                    vec![
                        MacroListElem::Parameter(1),
                        MacroListElem::Token(Token::Char(
                            'x',
                            Category::Letter
                        )),
                    ],
                    vec![
                        MacroListElem::Parameter(1),
                        MacroListElem::Token(Token::Char(
                            'y',
                            Category::Letter
                        )),
                        MacroListElem::Parameter(1),
                    ]
                )
            );
        });
    }

    #[test]
    fn it_sets_global_defs() {
        with_parser(&["\\global\\def\\a{x}%"], |parser| {
            parser.state.push_state();
            assert!(parser.is_assignment_head());
            parser.parse_assignment();
            assert_eq!(parser.lex_unexpanded_token(), None);
            parser.state.pop_state();

            assert_eq!(
                *parser
                    .state
                    .get_macro(&Token::ControlSequence("a".to_string()))
                    .unwrap(),
                Macro::new(
                    vec![],
                    vec![MacroListElem::Token(Token::Char(
                        'x',
                        Category::Letter
                    )),]
                )
            );
        });
    }

    #[test]
    fn it_assigns_lets_for_characters() {
        with_parser(&["\\let\\a=b%"], |parser| {
            parser.parse_assignment();

            assert_eq!(
                parser.state.get_renamed_token(&Token::ControlSequence(
                    "a".to_string()
                )),
                Some(Token::Char('b', Category::Letter))
            );
        });
    }

    #[test]
    fn it_assigns_lets_for_previously_defined_macros() {
        with_parser(&["\\def\\a{x}%", "\\let\\b=\\a%"], |parser| {
            parser.parse_assignment();
            parser.parse_assignment();

            assert_eq!(
                *parser
                    .state
                    .get_macro(&Token::ControlSequence("b".to_string()))
                    .unwrap(),
                Macro::new(
                    vec![],
                    vec![MacroListElem::Token(Token::Char(
                        'x',
                        Category::Letter
                    )),]
                )
            );
        });
    }

    #[test]
    fn it_doesnt_assign_lets_for_active_tokens() {
        with_parser(&["\\let\\a=@%"], |parser| {
            parser.state.set_category(false, '@', Category::Active);
            parser.parse_assignment();

            assert_eq!(
                parser.state.get_renamed_token(&Token::ControlSequence(
                    "a".to_string()
                )),
                None
            );
        });
    }

    #[test]
    fn it_assigns_lets_for_primitives() {
        with_parser(&["\\let\\a=\\def%"], |parser| {
            parser.parse_assignment();

            assert!(parser.state.is_token_equal_to_prim(
                &Token::ControlSequence("a".to_string()),
                "def"
            ));
        });
    }

    #[test]
    fn it_lets_let_be_let() {
        with_parser(&["\\let\\a=\\let%", "\\a\\x=y%"], |parser| {
            parser.parse_assignment();
            parser.parse_assignment();

            assert_eq!(
                parser.state.get_renamed_token(&Token::ControlSequence(
                    "x".to_string()
                )),
                Some(Token::Char('y', Category::Letter))
            );
        });
    }

    #[test]
    fn it_lets_def_be_let() {
        with_parser(&["\\let\\a=\\def%", "\\a\\x #1{#1}%"], |parser| {
            parser.parse_assignment();
            parser.parse_assignment();

            assert_eq!(
                *parser
                    .state
                    .get_macro(&Token::ControlSequence("x".to_string()))
                    .unwrap(),
                Macro::new(
                    vec![MacroListElem::Parameter(1),],
                    vec![MacroListElem::Parameter(1),]
                )
            );
        });
    }

    #[test]
    fn it_sets_global_lets() {
        with_parser(&["\\global\\let\\a=b%"], |parser| {
            parser.state.push_state();
            parser.parse_assignment();
            parser.state.pop_state();

            assert_eq!(
                parser.state.get_renamed_token(&Token::ControlSequence(
                    "a".to_string()
                )),
                Some(Token::Char('b', Category::Letter))
            );
        });
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

    #[test]
    fn it_parses_arithmetic() {
        with_parser(
            &[
                "\\count0=150%",
                "\\count1=5%",
                "\\advance\\count0 by7%",
                "\\multiply\\count1 by2%",
                "\\divide\\count0 by\\count1%",
            ],
            |parser| {
                parser.parse_assignment();
                parser.parse_assignment();

                assert_eq!(parser.state.get_count(0), 150);
                assert_eq!(parser.state.get_count(1), 5);

                assert!(parser.is_assignment_head());
                parser.parse_assignment();
                assert_eq!(parser.state.get_count(0), 157);

                assert!(parser.is_assignment_head());
                parser.parse_assignment();
                assert_eq!(parser.state.get_count(1), 10);

                assert!(parser.is_assignment_head());
                parser.parse_assignment();
                assert_eq!(parser.state.get_count(0), 15);
            },
        );
    }
}

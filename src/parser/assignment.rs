use std::rc::Rc;

use crate::category::Category;
use crate::math_code::MathCode;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_variable_assignment_head(&mut self) -> bool {
        self.is_integer_variable_head() || self.is_dimen_variable_head()
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

    fn is_shorthand_definition_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["mathchardef"])
    }

    fn is_code_assignment_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["mathcode"])
    }

    fn is_simple_assignment_head(&mut self) -> bool {
        self.is_let_assignment_head()
            || self.is_variable_assignment_head()
            || self.is_arithmetic_head()
            || self.is_box_assignment_head()
            || self.is_shorthand_definition_head()
            || self.is_code_assignment_head()
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
        if self.is_integer_variable_head() {
            let variable = self.parse_integer_variable();
            self.parse_equals_expanded();
            let value = self.parse_number();
            variable.set(self.state, global, value);
        } else if self.is_dimen_variable_head() {
            let variable = self.parse_dimen_variable();
            self.parse_equals_expanded();
            let value = self.parse_dimen();
            variable.set(self.state, global, value);
        } else {
            panic!("unimplemented");
        }
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
        let variable = self.parse_integer_variable();
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
        } else {
            panic!("unimplemented");
        }
    }

    fn is_box_assignment_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["setbox"])
    }

    fn parse_box_assignment(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if !self.state.is_token_equal_to_prim(&tok, "setbox") {
            panic!("Invalid box assignment head: {:?}", tok);
        }

        let box_index = self.parse_8bit_number();
        self.parse_equals_expanded();
        let maybe_tex_box = self.parse_box();

        if let Some(tex_box) = maybe_tex_box {
            self.state.set_box(global, box_index, tex_box);
        }
    }

    fn parse_shorthand_definition(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&tok, "mathchardef") {
            let control_sequence = self.parse_unexpanded_control_sequence();
            self.parse_equals_expanded();
            let code_value = self.parse_15bit_number();

            self.state.set_math_chardef(
                global,
                &control_sequence,
                &MathCode::from_number(code_value as u32),
            );
        } else {
            panic!("unimplemented!");
        }
    }

    fn parse_code_assignment(&mut self, global: bool) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&tok, "mathcode") {
            let num = self.parse_8bit_number();
            self.parse_equals_expanded();
            let code_value = self.parse_number();

            self.state.set_math_code(
                global,
                num as char,
                &MathCode::from_number(code_value as u32),
            );
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_simple_assignment(&mut self, global: bool) {
        if self.is_variable_assignment_head() {
            self.parse_variable_assignment(global)
        } else if self.is_let_assignment_head() {
            self.parse_let_assignment(global)
        } else if self.is_arithmetic_head() {
            self.parse_arithmetic(global)
        } else if self.is_box_assignment_head() {
            self.parse_box_assignment(global)
        } else if self.is_shorthand_definition_head() {
            self.parse_shorthand_definition(global)
        } else if self.is_code_assignment_head() {
            self.parse_code_assignment(global)
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
    use crate::dimension::{Dimen, Unit};
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

    #[test]
    fn it_sets_boxes() {
        with_parser(&["\\setbox123=\\hbox{a}%"], |parser| {
            assert!(parser.is_assignment_head());
            parser.parse_assignment();

            assert!(parser.state.get_box(123).is_some());
        });
    }

    #[test]
    fn it_sets_box_dimens() {
        with_parser(
            &[r"\setbox0=\hbox{a}%", r"\wd0=2pt%", r"\ht0=3pt%"],
            |parser| {
                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert_eq!(
                    parser.state.with_box(0, |tex_box| tex_box.width().clone()),
                    Some(Dimen::from_unit(2.0, Unit::Point))
                );
                assert_eq!(
                    parser
                        .state
                        .with_box(0, |tex_box| tex_box.height().clone()),
                    Some(Dimen::from_unit(3.0, Unit::Point))
                );
            },
        );
    }

    #[test]
    fn it_sets_mathchardefs() {
        with_parser(
            &[
                r#"\mathchardef\x"7161%"#,
                r"\mathchardef\y=1234%",
                r"\def\a{=}\mathchardef\z\a7161%",
                r"\x\y\z%",
            ],
            |parser| {
                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert!(parser.is_assignment_head());
                parser.parse_assignment();
                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                let x = parser.parse_unexpanded_control_sequence();
                let y = parser.parse_unexpanded_control_sequence();
                let z = parser.parse_unexpanded_control_sequence();

                assert_eq!(
                    parser.state.get_math_chardef(&x),
                    Some(MathCode::from_number(0x7161))
                );
                assert_eq!(
                    parser.state.get_math_chardef(&y),
                    Some(MathCode::from_number(1234))
                );
                assert_eq!(
                    parser.state.get_math_chardef(&z),
                    Some(MathCode::from_number(7161))
                );
            },
        );
    }

    #[test]
    fn it_sets_mathcodes() {
        with_parser(
            &[r#"\mathcode`*="2203%"#, r#"\mathcode`<="313C%"#],
            |parser| {
                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert!(parser.is_assignment_head());
                parser.parse_assignment();

                assert_eq!(
                    parser.state.get_math_code('*'),
                    MathCode::from_number(0x2203)
                );
                assert_eq!(
                    parser.state.get_math_code('<'),
                    MathCode::from_number(0x313C)
                );
            },
        );
    }
}

use crate::parser::Parser;
use crate::variable::{DimenVariable, IntegerVariable};

impl<'a> Parser<'a> {
    pub fn is_integer_variable_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["count"])
    }

    pub fn parse_integer_variable(&mut self) -> IntegerVariable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "count") {
            let index = self.parse_8bit_number();
            IntegerVariable::CountRegister(index)
        } else {
            panic!("unimplemented");
        }
    }

    pub fn is_dimen_variable_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["wd", "ht", "dp"])
    }

    pub fn parse_dimen_variable(&mut self) -> DimenVariable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "wd") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxWidth(index)
        } else if self.state.is_token_equal_to_prim(&token, "ht") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxHeight(index)
        } else if self.state.is_token_equal_to_prim(&token, "dp") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxDepth(index)
        } else {
            panic!("unimplemented");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::with_parser;

    #[test]
    fn it_parses_count_variables() {
        with_parser(
            &["\\let\\x=\\count%", "\\count0%", "\\count255%", "\\x255%"],
            |parser| {
                parser.parse_assignment(None);

                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(0)
                );
                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(255)
                );
                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(255)
                );
            },
        );
    }

    #[test]
    fn it_parses_box_dimen_variables() {
        with_parser(&["\\wd0%", "\\ht255%", "\\dp123%"], |parser| {
            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxWidth(0)
            );

            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxHeight(255)
            );

            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxDepth(123)
            );
        });
    }
}

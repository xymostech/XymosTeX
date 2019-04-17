use crate::parser::Parser;
use crate::variable::IntegerVariable;

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
                parser.parse_assignment();

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
}

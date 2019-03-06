use crate::parser::Parser;
use crate::variable::Variable;

impl<'a> Parser<'a> {
    pub fn is_variable_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => self.state.is_token_equal_to_prim(&token, "count"),
            _ => false,
        }
    }

    pub fn parse_variable(&mut self) -> Variable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "count") {
            let index = self.parse_8bit_number();
            Variable::Count(index)
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

                assert!(parser.is_variable_head());
                assert_eq!(parser.parse_variable(), Variable::Count(0));
                assert!(parser.is_variable_head());
                assert_eq!(parser.parse_variable(), Variable::Count(255));
                assert!(parser.is_variable_head());
                assert_eq!(parser.parse_variable(), Variable::Count(255));
            },
        );
    }
}

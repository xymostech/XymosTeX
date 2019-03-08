use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn is_print_head(&mut self) -> bool {
        match self.peek_unexpanded_token() {
            Some(token) => self.state.is_token_equal_to_prim(&token, "number"),
            _ => false,
        }
    }

    fn print_number(&mut self, value: i32) -> Vec<Token> {
        // Turn a number into Char tokens by taking advantage of rust's
        // built-in printing.
        value
            .to_string()
            .chars()
            .map(|chr| Token::Char(chr, Category::Other))
            .collect()
    }

    pub fn expand_print(&mut self) -> Vec<Token> {
        let head = self.lex_unexpanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&head, "number") {
            let value = self.parse_number_value();
            self.print_number(value)
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
    fn it_expands_numbers() {
        with_parser(
            &[
                "\\number0 %",
                "\\number12345 %",
                "\\number-1 %",
                "\\number\\count1%",
            ],
            |parser| {
                parser.state.set_count(false, 1, 123);

                // 0
                assert!(parser.is_print_head());
                assert_eq!(
                    parser.expand_print(),
                    vec![Token::Char('0', Category::Other)]
                );

                // 12345
                assert!(parser.is_print_head());
                assert_eq!(
                    parser.expand_print(),
                    vec![
                        Token::Char('1', Category::Other),
                        Token::Char('2', Category::Other),
                        Token::Char('3', Category::Other),
                        Token::Char('4', Category::Other),
                        Token::Char('5', Category::Other),
                    ]
                );

                // -1
                assert!(parser.is_print_head());
                assert_eq!(
                    parser.expand_print(),
                    vec![
                        Token::Char('-', Category::Other),
                        Token::Char('1', Category::Other),
                    ]
                );

                // \count1 = 123
                assert!(parser.is_print_head());
                assert_eq!(
                    parser.expand_print(),
                    vec![
                        Token::Char('1', Category::Other),
                        Token::Char('2', Category::Other),
                        Token::Char('3', Category::Other),
                    ]
                );
            },
        );
    }
}

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

pub fn is_token_digit(token: &Token) -> bool {
    match token {
        Token::Char(ch, Category::Other) => *ch >= '0' && *ch <= '9',
        _ => false,
    }
}

pub fn token_digit_value(token: &Token) -> u8 {
    if let Token::Char(ch, Category::Other) = token {
        if *ch >= '0' && *ch <= '9' {
            (*ch as u8) - ('0' as u8)
        } else {
            unreachable!();
        }
    } else {
        unreachable!();
    }
}

impl<'a> Parser<'a> {
    fn is_integer_constant_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => is_token_digit(&token),
            _ => false,
        }
    }

    fn parse_integer_constant(&mut self) -> u32 {
        let mut value: u32 = match self.peek_expanded_token() {
            Some(ref token) if is_token_digit(token) => {
                self.lex_expanded_token();
                token_digit_value(token) as u32
            }
            _ => panic!("Invalid number start"),
        };

        loop {
            match self.peek_expanded_token() {
                Some(ref token) if is_token_digit(token) => {
                    self.lex_expanded_token();
                    value = 10 * value + token_digit_value(token) as u32;
                }
                _ => break,
            }
        }

        self.parse_optional_space_expanded();

        value
    }

    pub fn is_internal_integer_head(&mut self) -> bool {
        self.is_variable_head()
    }

    pub fn parse_internal_integer(&mut self) -> i32 {
        if self.is_variable_head() {
            let variable = self.parse_variable();
            variable.get(self.state)
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_normal_integer(&mut self) -> i32 {
        if self.is_internal_integer_head() {
            self.parse_internal_integer()
        } else if self.is_integer_constant_head() {
            self.parse_integer_constant() as i32
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_unsigned_number(&mut self) -> i32 {
        self.parse_normal_integer()
    }

    // Parses some number of +s and -s into an overall numeric sign, which is
    // -1 if there are an odd number of -s and 1 otherwise.
    pub fn parse_optional_signs(&mut self) -> i32 {
        let mut sign: i32 = 1;

        loop {
            match self.peek_expanded_token() {
                Some(Token::Char(chr, Category::Other))
                    if chr == '+' || chr == '-' =>
                {
                    self.lex_expanded_token();
                    if chr == '-' {
                        sign *= -1;
                    }
                }
                _ => break,
            }
        }

        self.parse_optional_spaces_expanded();

        sign
    }

    pub fn parse_8bit_number(&mut self) -> u8 {
        let number = self.parse_number();
        if number < 0 || number > 255 {
            panic!("Invalid 8-bit number: {}", number);
        }
        number as u8
    }

    pub fn parse_number(&mut self) -> i32 {
        let sign = self.parse_optional_signs();
        let value = self.parse_unsigned_number();
        sign * value
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::with_parser;

    #[test]
    fn it_parses_8bit_numbers() {
        with_parser(&["0 %", "255 %", "-+-  123 %"], |parser| {
            assert_eq!(parser.parse_8bit_number(), 0);
            assert_eq!(parser.parse_8bit_number(), 255);
            assert_eq!(parser.parse_8bit_number(), 123);
        });
    }

    #[test]
    #[should_panic(expected = "Invalid 8-bit number: -1234")]
    fn it_fails_parsing_8bit_numbers() {
        with_parser(&["-1234%"], |parser| {
            parser.parse_8bit_number();
        });
    }

    #[test]
    fn it_parses_numbers_from_variables() {
        with_parser(&["\\count10%"], |parser| {
            parser.state.set_count(false, 10, 1234);
            assert_eq!(parser.parse_number(), 1234);
        });
    }

    #[test]
    fn it_parses_negative_integer_variables() {
        with_parser(&["-\\count10%"], |parser| {
            parser.state.set_count(false, 10, 1234);
            assert_eq!(parser.parse_number(), -1234);
        });
    }
}

use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

pub fn is_token_digit(token: &Token) -> bool {
    match token {
        Token::Char(ch, Category::Other) => *ch >= '0' && *ch <= '9',
        _ => false,
    }
}

pub fn is_token_hex_digit(token: &Token) -> bool {
    match token {
        Token::Char(ch, Category::Other) => {
            (*ch >= '0' && *ch <= '9') || (*ch >= 'A' && *ch <= 'F')
        }
        Token::Char(ch, Category::Letter) => *ch >= 'A' && *ch <= 'F',
        _ => false,
    }
}

pub fn token_digit_value(token: &Token) -> u8 {
    if let Token::Char(ch, Category::Other) = token {
        if *ch >= '0' && *ch <= '9' {
            (*ch as u8) - ('0' as u8)
        } else if *ch >= 'A' && *ch <= 'F' {
            (*ch as u8) - ('A' as u8) + 10
        } else {
            panic!("Invalid token digit: {}", ch);
        }
    } else if let Token::Char(ch, Category::Letter) = token {
        if *ch >= 'A' && *ch <= 'F' {
            (*ch as u8) - ('A' as u8) + 10
        } else {
            panic!("Invalid token digit: {}", ch);
        }
    } else {
        panic!("Invalid token digit: {:?}", token);
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

    fn is_hexadecimal_constant_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => token == Token::Char('"', Category::Other),
            _ => false,
        }
    }

    fn parse_hexadecimal_constant(&mut self) -> u32 {
        let quote = self.lex_expanded_token().unwrap();
        if quote != Token::Char('"', Category::Other) {
            panic!("Invalid hexadecimal number start");
        }

        let mut value: u32 = match self.peek_expanded_token() {
            Some(ref token) if is_token_hex_digit(token) => {
                self.lex_expanded_token();
                token_digit_value(token) as u32
            }
            _ => panic!("Invalid hexadecimal number start"),
        };

        loop {
            match self.peek_expanded_token() {
                Some(ref token) if is_token_hex_digit(token) => {
                    self.lex_expanded_token();
                    value = 16 * value + token_digit_value(token) as u32;
                }
                _ => break,
            }
        }

        self.parse_optional_space_expanded();

        value
    }

    pub fn is_internal_integer_head(&mut self) -> bool {
        self.is_integer_variable_head()
    }

    pub fn parse_internal_integer(&mut self) -> i32 {
        if self.is_integer_variable_head() {
            let variable = self.parse_integer_variable();
            variable.get(self.state)
        } else {
            panic!("unimplemented");
        }
    }

    fn is_normal_integer_head(&mut self) -> bool {
        self.is_internal_integer_head()
            || self.is_integer_constant_head()
            || self.is_hexadecimal_constant_head()
    }

    fn parse_normal_integer(&mut self) -> i32 {
        if self.is_internal_integer_head() {
            self.parse_internal_integer()
        } else if self.is_integer_constant_head() {
            self.parse_integer_constant() as i32
        } else if self.is_hexadecimal_constant_head() {
            self.parse_hexadecimal_constant() as i32
        } else {
            panic!("unimplemented");
        }
    }

    fn is_coerced_integer_head(&mut self) -> bool {
        self.is_internal_dimen_head()
    }

    fn parse_coerced_integer(&mut self) -> i32 {
        let dimen = self.parse_internal_dimen();
        dimen.as_scaled_points()
    }

    fn parse_unsigned_number(&mut self) -> i32 {
        if self.is_normal_integer_head() {
            self.parse_normal_integer()
        } else if self.is_coerced_integer_head() {
            self.parse_coerced_integer()
        } else {
            panic!("Invalid unsigned number head");
        }
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

    pub fn parse_15bit_number(&mut self) -> u16 {
        let number = self.parse_number();
        if number < 0 || number > 32767 {
            panic!("Invalid 15-bit number: {}", number);
        }
        number as u16
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

    #[test]
    fn it_parses_hexadecimal_numbers() {
        with_parser(&["\"0", "\"1289", "\"ABEF", "\"F0F"], |parser| {
            assert_eq!(parser.parse_number(), 0x0);
            assert_eq!(parser.parse_number(), 0x1289);
            assert_eq!(parser.parse_number(), 0xABEF);
            assert_eq!(parser.parse_number(), 0xF0F);
        });
    }

    #[test]
    fn it_parses_coerced_dimens() {
        with_parser(&[r"\setbox0=\hbox{g}%", r"\wd0%", r"-\ht0%"], |parser| {
            parser.parse_assignment();

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

            assert_eq!(
                parser.parse_number(),
                metrics.get_width('g').as_scaled_points()
            );
            assert_eq!(
                parser.parse_number(),
                -1 * metrics.get_height('g').as_scaled_points()
            );
        });
    }
}

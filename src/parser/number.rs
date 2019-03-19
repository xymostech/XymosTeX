use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

fn is_token_digit(token: &Token) -> bool {
    match token {
        Token::Char(ch, Category::Other) => *ch >= '0' && *ch <= '9',
        _ => false,
    }
}

fn token_digit_value(token: &Token) -> u8 {
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

    pub fn parse_unsigned_number(&mut self) -> u32 {
        // TODO(xymostech): this removes the distinction between "normal" and
        // "coerced" integers. Fix that
        if self.is_integer_constant_head() {
            self.parse_integer_constant()
        } else {
            panic!("unimplemented");
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

    fn parse_number(&mut self) -> i32 {
        let sign = self.parse_optional_signs();
        let number = self.parse_unsigned_number();
        sign * (number as i32)
    }

    pub fn parse_8bit_number(&mut self) -> u8 {
        let number = self.parse_number();
        if number < 0 || number > 255 {
            panic!("Invalid 8-bit number: {}", number);
        }
        number as u8
    }

    pub fn parse_number_value(&mut self) -> i32 {
        if self.is_variable_head() {
            let variable = self.parse_variable();
            variable.get(self.state)
        } else {
            self.parse_number()
        }
    }
}

/**
 * This file contains Parser functions which parse commonly used constructs in
 * the TeX grammar.
 */
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
    // Parse 0 or more space tokens and ignore them
    pub fn parse_optional_spaces_unexpanded(&mut self) {
        while let Some(Token::Char(_, Category::Space)) =
            self.peek_unexpanded_token()
        {
            self.lex_unexpanded_token();
        }
    }

    // Parse 0 or more expanded space tokens and ignore them
    pub fn parse_optional_spaces_expanded(&mut self) {
        while let Some(Token::Char(_, Category::Space)) =
            self.peek_expanded_token()
        {
            self.lex_expanded_token();
        }
    }

    // Parse 0 or 1 space tokens and ignore them
    pub fn parse_optional_space_unexpanded(&mut self) {
        if let Some(Token::Char(_, Category::Space)) =
            self.peek_unexpanded_token()
        {
            self.lex_unexpanded_token();
        }
    }

    // Parse 0 or 1 expanded space tokens and ignore them
    pub fn parse_optional_space_expanded(&mut self) {
        if let Some(Token::Char(_, Category::Space)) =
            self.peek_expanded_token()
        {
            self.lex_expanded_token();
        }
    }

    // Parses an <equals>
    pub fn parse_equals_unexpanded(&mut self) {
        self.parse_optional_spaces_unexpanded();
        if let Some(Token::Char('=', Category::Other)) =
            self.peek_unexpanded_token()
        {
            self.lex_unexpanded_token();
        }
    }

    fn is_integer_constant_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => is_token_digit(&token),
            _ => false,
        }
    }

    fn parse_integer_constant(&mut self) -> u64 {
        let mut value: u64 = match self.peek_expanded_token() {
            Some(ref token) if is_token_digit(token) => {
                self.lex_expanded_token();
                token_digit_value(token) as u64
            }
            _ => panic!("Invalid number start"),
        };

        loop {
            match self.peek_expanded_token() {
                Some(ref token) if is_token_digit(token) => {
                    self.lex_expanded_token();
                    value = 10 * value + token_digit_value(token) as u64;
                }
                _ => break,
            }
        }

        self.parse_optional_space_expanded();

        value
    }

    fn parse_unsigned_number(&mut self) -> u64 {
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
    fn parse_optional_signs(&mut self) -> i64 {
        let mut sign: i64 = 1;

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

    fn parse_number(&mut self) -> i64 {
        let sign = self.parse_optional_signs();
        let number = self.parse_unsigned_number();
        sign * (number as i64)
    }

    pub fn parse_8bit_number(&mut self) -> u8 {
        let number = self.parse_number();
        if number < 0 || number > 255 {
            panic!("Invalid 8-bit number: {}", number);
        }
        number as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::state::TeXState;

    fn with_parser<T>(lines: &[&str], cb: T)
    where
        T: FnOnce(&mut Parser),
    {
        let state = TeXState::new();
        let mut parser = Parser::new(lines, &state);

        cb(&mut parser);
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_parses_an_optional_space() {
        // Testing a single, unexpanded optional space
        with_parser(&["a aa%"], |parser| {
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_space_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_space_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });

        // Now with an expanded space
        with_parser(&["\\def\\x{ }%", "a a\\x aa%"], |parser| {
            parser.parse_assignment();

            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_space_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_space_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_space_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_parses_optional_spaces() {
        // Testing multiple optional unexpanded spaces (kinda, except that it's
        // hard to get multiple spaces in a row because they don't come out of
        // the lexer, so we only test one space).
        with_parser(&["a aa%"], |parser| {
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });

        // Testing multiple optional expanded spaces
        with_parser(&["\\def\\x{ }%", "aa a \\x a\\x\\x a%"], |parser| {
            parser.parse_assignment();

            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_optional_spaces_expanded();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_parses_equals_signs() {
        with_parser(&["a=a  =a%"], |parser| {
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_equals_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            parser.parse_equals_unexpanded();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_parses_8bit_numbers() {
        with_parser(&["0 %", "255 %", "-+-  123 %"], |parser| {
            assert_eq!(parser.parse_8bit_number(), 0);
            assert_eq!(parser.parse_8bit_number(), 255);
            assert_eq!(parser.parse_8bit_number(), 123);
        });
    }
}

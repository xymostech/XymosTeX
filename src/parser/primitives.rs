/**
 * This file contains Parser functions which parse commonly used constructs in
 * the TeX grammar.
 */
use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

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

    // Parses an <equals> without expanding tokens
    pub fn parse_equals_unexpanded(&mut self) {
        self.parse_optional_spaces_unexpanded();
        if let Some(Token::Char('=', Category::Other)) =
            self.peek_unexpanded_token()
        {
            self.lex_unexpanded_token();
        }
    }

    // Parses an <equals> while expanding tokens
    pub fn parse_equals_expanded(&mut self) {
        self.parse_optional_spaces_expanded();
        if let Some(Token::Char('=', Category::Other)) =
            self.peek_expanded_token()
        {
            self.lex_expanded_token();
        }
    }

    pub fn parse_optional_keyword_expanded(&mut self, keyword: &str) {
        self.parse_optional_spaces_expanded();

        let first_char = keyword.chars().next().unwrap();

        // If the first token doesn't match the keyword we're looking for, just
        // bail out now.
        match self.peek_expanded_token() {
            Some(Token::Char(_, Category::Active)) => return,
            Some(Token::Char(ch, _))
                if ch == first_char.to_ascii_lowercase()
                    || ch == first_char.to_ascii_uppercase() =>
            {
                ()
            }
            _ => return,
        }

        for keyword_char in keyword.chars() {
            let token = self.lex_expanded_token();
            match token {
                Some(Token::Char(_, Category::Active)) => panic!(
                    "Found invalid token {:?} while parsing keyword {}",
                    token, keyword
                ),
                Some(Token::Char(ch, _))
                    // TODO(xymostech): Handle non-ascii?
                    if ch == keyword_char.to_ascii_lowercase()
                        || ch == keyword_char.to_ascii_uppercase() =>
                {
                    ()
                }
                _ => panic!(
                    "Found invalid token {:?} while parsing keyword {}",
                    token, keyword
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::with_parser;

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

    #[test]
    #[should_panic(expected = "Invalid 8-bit number: -1234")]
    fn it_fails_parsing_8bit_numbers() {
        with_parser(&["-1234%"], |parser| {
            parser.parse_8bit_number();
        });
    }

    #[test]
    fn it_parses_number_values_from_variables() {
        with_parser(&["\\count10%"], |parser| {
            parser.state.set_count(false, 10, 1234);
            assert_eq!(parser.parse_number_value(), 1234);
        });
    }

    #[test]
    fn it_parses_optional_keywords() {
        with_parser(&["   by    pt   TrUe%"], |parser| {
            parser.parse_optional_keyword_expanded("by");

            parser.parse_optional_keyword_expanded("by");
            parser.parse_optional_keyword_expanded("pt");

            parser.parse_optional_keyword_expanded("pt");
            parser.parse_optional_keyword_expanded("true");
        });
    }

    #[test]
    #[should_panic(expected = "while parsing keyword")]
    fn it_fails_parsing_halfway_through_a_keyword() {
        with_parser(&[" boo%"], |parser| {
            parser.parse_optional_keyword_expanded("by");
        });
    }
}

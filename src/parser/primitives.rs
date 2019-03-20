/**
 * This file contains Parser functions which parse commonly used constructs in
 * the TeX grammar.
 */
use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

/// Checks if a token matches a character from a keyword. In particular, this
/// matches if it is a char token whose char component equals the the keyword
/// character case-insensitively.
pub fn token_equals_keyword_char(tok: &Token, ch: char) -> bool {
    match tok {
        Token::Char(_, Category::Active) => false,
        Token::Char(tok_ch, _) =>
        // TODO(xymostech): Handle non-ascii?
        {
            *tok_ch == ch.to_ascii_lowercase()
                || *tok_ch == ch.to_ascii_uppercase()
        }
        _ => false,
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

    /// Parses a keyword (which is indicated in the TeXbook grammar by letters
    /// in typewriter font). Panics if the keyword is not immediately found.
    pub fn parse_keyword_expanded(&mut self, keyword: &str) {
        self.parse_optional_spaces_expanded();

        for keyword_char in keyword.chars() {
            let token = self.lex_expanded_token();
            match token {
                Some(ref tok)
                    if token_equals_keyword_char(tok, keyword_char) =>
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

    /// Parses an optional keyword. Panics if the keyword is only partially
    /// found.
    pub fn parse_optional_keyword_expanded(&mut self, keyword: &str) {
        self.parse_optional_spaces_expanded();

        let first_char = keyword.chars().next().unwrap();

        // If the first token doesn't match the keyword we're looking for, just
        // bail out now.
        match self.peek_expanded_token() {
            Some(ref tok) if token_equals_keyword_char(tok, first_char) => (),
            _ => return,
        }

        // Now that we're confident the keyword is actually there, parse it.
        self.parse_keyword_expanded(keyword);
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

    #[test]
    fn it_parses_required_keywords() {
        with_parser(&[" pt  minus  fillll%"], |parser| {
            parser.parse_keyword_expanded("pt");
            parser.parse_keyword_expanded("minus");
            parser.parse_keyword_expanded("fillll");
        });
    }

    #[test]
    #[should_panic(expected = "while parsing keyword")]
    fn it_fails_parsing_missing_required_keywords() {
        with_parser(&[" blah"], |parser| {
            parser.parse_keyword_expanded("pt");
        });
    }
}

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

    /// Parses an optional keyword. Returns true if the keyword was parsed and
    /// false if not.
    pub fn parse_optional_keyword_expanded(&mut self, keyword: &str) -> bool {
        self.parse_optional_spaces_expanded();

        let first_char = keyword.chars().next().unwrap();

        // If the first token doesn't match the keyword we're looking for, just
        // bail out now.
        match self.peek_expanded_token() {
            Some(ref tok) if token_equals_keyword_char(tok, first_char) => (),
            _ => return false,
        }

        // Now that we're confident the keyword is actually there, parse it.
        let mut parsed_toks = Vec::new();

        for keyword_char in keyword.chars() {
            let token = self.peek_expanded_token();
            match token {
                Some(ref tok)
                    if token_equals_keyword_char(tok, keyword_char) =>
                {
                    let tok = self.lex_expanded_token().unwrap();
                    parsed_toks.push(tok);
                }
                _ => {
                    self.add_upcoming_tokens(parsed_toks);
                    return false;
                }
            }
        }

        true
    }

    /// Parses a <filler>, which is any amount of spaces and \relax
    pub fn parse_filler_expanded(&mut self) {
        self.parse_optional_spaces_expanded();
        loop {
            match self.peek_expanded_token() {
                None => break,
                Some(token) => {
                    if self.state.is_token_equal_to_prim(&token, "relax") {
                        self.lex_expanded_token();
                    } else {
                        break;
                    }
                }
            }
            self.parse_optional_spaces_expanded();
        }
    }

    pub fn is_next_expanded_token_in_set_of_primitives(
        &mut self,
        primitives: &[&str],
    ) -> bool {
        match self.peek_expanded_token() {
            Some(token) => primitives
                .iter()
                .any(|prim| self.state.is_token_equal_to_prim(&token, prim)),
            _ => false,
        }
    }

    /// Given a (maybe) token, returns the token that the token is potentially
    /// renamed to. For instance, after
    ///   \let\bgroup={
    /// then
    ///   parser.replace_renamed_token(
    ///       Some(Token::ControlSequence("bgroup".to_string())))
    /// will return
    ///   Some(Token::Char('{', Category::BeginGroup))
    /// This is explicitly not part of token expansion.
    pub fn replace_renamed_token(
        &mut self,
        maybe_token: Option<Token>,
    ) -> Option<Token> {
        maybe_token
            .map(|token| self.state.get_renamed_token(&token).unwrap_or(token))
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
            assert_eq!(parser.parse_optional_keyword_expanded("by"), true);

            assert_eq!(parser.parse_optional_keyword_expanded("by"), false);
            assert_eq!(parser.parse_optional_keyword_expanded("pc"), false);
            assert_eq!(parser.parse_optional_keyword_expanded("pt"), true);

            assert_eq!(parser.parse_optional_keyword_expanded("pt"), false);
            assert_eq!(parser.parse_optional_keyword_expanded("trust"), false);
            assert_eq!(parser.parse_optional_keyword_expanded("true"), true);
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

    #[test]
    fn it_fetches_renamed_tokens() {
        with_parser(&[r"\let\bgroup={%", r"\bgroup"], |parser| {
            parser.parse_assignment();

            let unreplaced = parser.lex_unexpanded_token();
            assert_eq!(
                parser.replace_renamed_token(unreplaced),
                Some(Token::Char('{', Category::BeginGroup))
            );
        });
    }
}

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
        while let Some(Token::Char(_, Category::Space)) = self.peek_unexpanded_token() {
            self.lex_unexpanded_token();
        }
    }

    // Parse 0 or 1 space tokens and ignore them
    pub fn parse_optional_space_unexpanded(&mut self) {
        if let Some(Token::Char(_, Category::Space)) = self.peek_unexpanded_token() {
            self.lex_unexpanded_token();
        }
    }

    // Parses an <equals>
    pub fn parse_equals_unexpanded(&mut self) {
        self.parse_optional_spaces_unexpanded();
        if let Some(Token::Char('=', Category::Other)) = self.peek_unexpanded_token() {
            self.lex_unexpanded_token();
        }
    }
}

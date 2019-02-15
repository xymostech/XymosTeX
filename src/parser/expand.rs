use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn lex_expanded_token(&mut self) -> Option<Token> {
        let tok = self.lex_unexpanded_token();

        tok
    }

    pub fn peek_expanded_token(&mut self) -> Option<Token> {
        match self.lex_expanded_token() {
            Some(token) => {
                self.add_upcoming_token(token.clone());
                Some(token)
            }
            None => None,
        }
    }

    pub fn lex_unexpanded_token(&mut self) -> Option<Token> {
        if self.upcoming_tokens.is_empty() {
            self.lexer.lex_token()
        } else {
            self.upcoming_tokens.pop()
        }
    }

    fn add_upcoming_token(&mut self, token: Token) {
        self.upcoming_tokens.push(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::Category;
    use crate::state::TeXState;

    #[test]
    fn it_lexes_tokens() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["a%"], &state);

        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_peeks_tokens() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["a%"], &state);

        assert_eq!(
            parser.peek_expanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(parser.lex_unexpanded_token(), None);
    }
}

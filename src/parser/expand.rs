use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn lex_expanded_token(&mut self) -> Option<Token> {
        match self.lex_unexpanded_token() {
            None => None,
            Some(token) => match self.state.get_macro(&token) {
                None => Some(token),
                Some(makro) => {
                    let replacement_map = self.parse_replacement_map(&makro);
                    let replacement = makro.get_replacement(&replacement_map);
                    self.add_upcoming_tokens(replacement);
                    self.lex_expanded_token()
                }
            },
        }
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

    pub fn peek_unexpanded_token(&mut self) -> Option<Token> {
        match self.lex_unexpanded_token() {
            Some(token) => {
                self.add_upcoming_token(token.clone());
                Some(token)
            }
            None => None,
        }
    }

    // Sometimes, we need to undo the lexing of a token. This function accepts
    // a token that we want to lex next. This undoing happens in a few places:
    //  * When we're peeking at tokens (e.g. when we're handling <optional
    //    spaces> and we want to check if the next token is a space)
    //  * When we expand something, so we want the next lexed tokens to be the
    //    expanded result
    fn add_upcoming_token(&mut self, token: Token) {
        self.upcoming_tokens.push(token);
    }

    fn add_upcoming_tokens(&mut self, tokens: Vec<Token>) {
        for token in tokens.into_iter().rev() {
            self.add_upcoming_token(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::rc::Rc;

    use crate::category::Category;
    use crate::makro::{Macro, MacroListElem};
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
            parser.peek_unexpanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_expands_macros() {
        let state = TeXState::new();
        state.set_macro(
            Token::ControlSequence("a".to_string()),
            Rc::new(Macro::new(
                vec![MacroListElem::Parameter(1)],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Parameter(1),
                    MacroListElem::Parameter(1),
                ],
            )),
        );

        let mut parser = Parser::new(&["\\a{ab}%"], &state);

        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('b', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('b', Category::Letter))
        );
        assert_eq!(parser.lex_expanded_token(), None);
    }

    #[test]
    fn it_peeks_expanded_tokens() {
        let state = TeXState::new();
        state.set_macro(
            Token::ControlSequence("a".to_string()),
            Rc::new(Macro::new(
                vec![],
                vec![MacroListElem::Token(Token::Char('x', Category::Letter))],
            )),
        );

        let mut parser = Parser::new(&["\\a b%"], &state);

        assert_eq!(
            parser.peek_expanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(
            parser.peek_expanded_token(),
            Some(Token::Char('b', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('b', Category::Letter))
        );
        assert_eq!(parser.lex_expanded_token(), None);
    }
}

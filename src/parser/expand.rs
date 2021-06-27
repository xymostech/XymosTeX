use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn lex_expanded_token(&mut self) -> Option<Token> {
        if self.is_conditional_head() {
            // Handle conditionals, like \ifnum
            self.expand_conditional();
            return self.lex_expanded_token();
        } else if self.is_print_head() {
            // Handle printing, like \number\count1
            let replacement = self.expand_print();
            self.add_upcoming_tokens(replacement);
            return self.lex_expanded_token();
        }

        match self.lex_unexpanded_token() {
            None => None,
            Some(token) => {
                // Handle macro expansion
                if let Some(makro) = self.state.get_macro(&token) {
                    let replacement_map = self.parse_replacement_map(&makro);
                    let replacement = makro.get_replacement(&replacement_map);
                    self.add_upcoming_tokens(replacement);
                    self.lex_expanded_token()
                } else {
                    // Passthrough anything else
                    Some(token)
                }
            }
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
    //  * When we're following the instructions to "insert the token <tok> into
    //    the input", like we do when seeing vertical mode material in
    //    horizontal mode.
    //
    // Note: Use this function sparingly outside of this file! For efficiency's
    // sake, we should try to peek tokens instead of manually parsing and
    // un-parsing them.
    pub fn add_upcoming_token(&mut self, token: Token) {
        self.upcoming_tokens.push(token);
    }

    // Adds multiple tokens with add_upcoming_token(). We add the tokens in
    // reverse so that the first token in the list gets parsed next first.
    // Note: Use this function sparingly! For efficiency's sake, we should try
    // only peek one token ahead when we can.
    pub fn add_upcoming_tokens(&mut self, tokens: Vec<Token>) {
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
    use crate::testing::with_parser;

    #[test]
    fn it_lexes_tokens() {
        with_parser(&["a%"], |parser| {
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_peeks_tokens() {
        with_parser(&["a%"], |parser| {
            assert_eq!(
                parser.peek_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_expands_macros() {
        with_parser(&["\\a{ab}%"], |parser| {
            parser.state.set_macro(
                false,
                &Token::ControlSequence("a".to_string()),
                &Rc::new(Macro::new(
                    vec![MacroListElem::Parameter(1)],
                    vec![
                        MacroListElem::Token(Token::Char(
                            'x',
                            Category::Letter,
                        )),
                        MacroListElem::Parameter(1),
                        MacroListElem::Parameter(1),
                    ],
                )),
            );

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
        });
    }

    #[test]
    fn it_expands_conditionals() {
        with_parser(&["\\iftrue x\\else y\\fi%"], |parser| {
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('x', Category::Letter))
            );
            assert_eq!(parser.lex_expanded_token(), None,);
        });
    }

    #[test]
    fn it_peeks_expanded_tokens() {
        with_parser(&["\\a b%"], |parser| {
            parser.state.set_macro(
                false,
                &Token::ControlSequence("a".to_string()),
                &Rc::new(Macro::new(
                    vec![],
                    vec![MacroListElem::Token(Token::Char(
                        'x',
                        Category::Letter,
                    ))],
                )),
            );

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
        });
    }

    #[test]
    fn it_prints_numbers() {
        with_parser(&["\\count1=-100 %", "\\number\\count1%"], |parser| {
            parser.parse_assignment(None);
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('-', Category::Other))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('1', Category::Other))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('0', Category::Other))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('0', Category::Other))
            );
        });
    }
}

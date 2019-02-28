use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn is_conditional_head(&mut self) -> bool {
        match self.peek_unexpanded_token() {
            Some(token) => {
                self.state.is_token_equal_to_cs(&token, "else")
                    || self.state.is_token_equal_to_cs(&token, "fi")
                    || self.state.is_token_equal_to_cs(&token, "iftrue")
                    || self.state.is_token_equal_to_cs(&token, "iffalse")
            }
            _ => false,
        }
    }

    // Skips tokens until a \fi or \else is parsed. Returns true if the token
    // we found is \else, false if it is \fi.
    fn skip_to_fi_or_else(&mut self) -> bool {
        let mut ends_with_else = false;
        loop {
            let token = self.lex_unexpanded_token().unwrap();
            if self.state.is_token_equal_to_cs(&token, "fi") {
                break;
            } else if self.state.is_token_equal_to_cs(&token, "else") {
                ends_with_else = true;
                break;
            }
        }
        ends_with_else
    }

    fn skip_from_else(&mut self) {
        // When we encounter an \else, we know that we're in a 'true'
        // conditional because in a 'false' conditional, we always already
        // parse the \else token in skip_to_fi_or_else(). Thus, we just need to
        // skip tokens until we see a \fi.
        loop {
            let token = self.lex_unexpanded_token().unwrap();
            if self.state.is_token_equal_to_cs(&token, "fi") {
                break;
            }
        }
    }

    fn handle_true(&mut self) {
        self.conditional_depth += 1;
    }

    fn handle_false(&mut self) {
        if self.skip_to_fi_or_else() {
            // If we skipped all the way to a \fi, we don't add to our depth of
            // conditionals because we already exited this one. If we only
            // skipped to a \else, we are now inside a conditional.
            self.conditional_depth += 1;
        }
    }

    pub fn expand_conditional(&mut self) {
        let token = self.lex_unexpanded_token().unwrap();

        if self.state.is_token_equal_to_cs(&token, "fi") {
            if self.conditional_depth == 0 {
                panic!("Extra \\fi");
            }
            self.conditional_depth -= 1;
        } else if self.state.is_token_equal_to_cs(&token, "else") {
            if self.conditional_depth == 0 {
                panic!("Extra \\else");
            }
            self.conditional_depth -= 1;
            self.skip_from_else();
        } else if self.state.is_token_equal_to_cs(&token, "iftrue") {
            self.handle_true();
        } else if self.state.is_token_equal_to_cs(&token, "iffalse") {
            self.handle_false();
        } else {
            panic!("unimplemented");
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
    fn it_parses_single_body_iftrue() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\iftrue x\\fi%"], &state);

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_parses_iftrue_with_else() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\iftrue x\\else y\\fi%"], &state);

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_parses_single_body_iffalse() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\iffalse x\\fi%"], &state);

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_parses_iffalse_with_else() {
        let state = TeXState::new();
        let mut parser = Parser::new(&["\\iffalse x\\else y\\fi%"], &state);

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_unexpanded_token(),
            Some(Token::Char('y', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_expand_macros_in_true_bodies_but_not_false_bodies() {
        let state = TeXState::new();
        state.set_macro(
            false,
            &Token::ControlSequence("a".to_string()),
            &Rc::new(Macro::new(
                vec![],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Token(Token::ControlSequence(
                        "else".to_string(),
                    )),
                    MacroListElem::Token(Token::Char('y', Category::Letter)),
                ],
            )),
        );
        state.set_macro(
            false,
            &Token::ControlSequence("b".to_string()),
            &Rc::new(Macro::new(
                vec![],
                vec![
                    MacroListElem::Token(Token::Char('z', Category::Letter)),
                    MacroListElem::Token(Token::ControlSequence(
                        "fi".to_string(),
                    )),
                ],
            )),
        );
        let mut parser = Parser::new(&["\\iftrue w\\a\\b\\fi%"], &state);

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('w', Category::Letter))
        );
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('x', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(parser.lex_unexpanded_token(), None);
    }

    #[test]
    fn it_allows_conditional_primitives_to_be_let() {
        let state = TeXState::new();
        state.set_let(
            false,
            &Token::ControlSequence("iftruex".to_string()),
            &Token::ControlSequence("iftrue".to_string()),
        );
        state.set_let(
            false,
            &Token::ControlSequence("iffalsex".to_string()),
            &Token::ControlSequence("iffalse".to_string()),
        );
        state.set_let(
            false,
            &Token::ControlSequence("fix".to_string()),
            &Token::ControlSequence("fi".to_string()),
        );
        state.set_let(
            false,
            &Token::ControlSequence("elsex".to_string()),
            &Token::ControlSequence("else".to_string()),
        );
        let mut parser = Parser::new(
            &["\\iftruex a\\elsex b\\fix%", "\\iffalsex a\\elsex b\\fix%"],
            &state,
        );

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('a', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();

        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();
        assert_eq!(
            parser.lex_expanded_token(),
            Some(Token::Char('b', Category::Letter))
        );
        assert_eq!(parser.is_conditional_head(), true);
        parser.expand_conditional();

        assert_eq!(parser.lex_unexpanded_token(), None);
    }
}

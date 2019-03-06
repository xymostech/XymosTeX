use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

enum Relation {
    GreaterThan,
    EqualTo,
    LessThan,
}

fn check_relation(rel: Relation, left: i32, right: i32) -> bool {
    match rel {
        Relation::GreaterThan => left > right,
        Relation::EqualTo => left == right,
        Relation::LessThan => left < right,
    }
}

impl<'a> Parser<'a> {
    pub fn is_conditional_head(&mut self) -> bool {
        match self.peek_unexpanded_token() {
            Some(token) => {
                self.state.is_token_equal_to_prim(&token, "else")
                    || self.state.is_token_equal_to_prim(&token, "fi")
                    || self.state.is_token_equal_to_prim(&token, "iftrue")
                    || self.state.is_token_equal_to_prim(&token, "iffalse")
                    || self.state.is_token_equal_to_prim(&token, "ifnum")
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
            if self.state.is_token_equal_to_prim(&token, "fi") {
                break;
            } else if self.state.is_token_equal_to_prim(&token, "else") {
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
            if self.state.is_token_equal_to_prim(&token, "fi") {
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

    fn parse_relation(&mut self) -> Relation {
        let relation = match self.lex_expanded_token() {
            Some(Token::Char('<', Category::Other)) => Relation::LessThan,
            Some(Token::Char('=', Category::Other)) => Relation::EqualTo,
            Some(Token::Char('>', Category::Other)) => Relation::GreaterThan,
            rest => panic!("Invalid relation: {:?}", rest),
        };
        self.parse_optional_spaces_expanded();
        relation
    }

    pub fn expand_conditional(&mut self) {
        let token = self.lex_unexpanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "fi") {
            if self.conditional_depth == 0 {
                panic!("Extra \\fi");
            }
            self.conditional_depth -= 1;
        } else if self.state.is_token_equal_to_prim(&token, "else") {
            if self.conditional_depth == 0 {
                panic!("Extra \\else");
            }
            self.conditional_depth -= 1;
            self.skip_from_else();
        } else if self.state.is_token_equal_to_prim(&token, "iftrue") {
            self.handle_true();
        } else if self.state.is_token_equal_to_prim(&token, "iffalse") {
            self.handle_false();
        } else if self.state.is_token_equal_to_prim(&token, "ifnum") {
            let num1 = self.parse_number_value();
            let relation = self.parse_relation();
            let num2 = self.parse_number_value();

            if check_relation(relation, num1, num2) {
                self.handle_true();
            } else {
                self.handle_false();
            }
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
    use crate::testing::with_parser;

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

    #[test]
    fn it_parses_ifnum() {
        with_parser(
            &[
                "\\ifnum1<2 t\\else f\\fi%",
                "\\ifnum1>2 t\\else f\\fi%",
                "\\ifnum1=2 t\\else f\\fi%",
            ],
            |parser| {
                // 1<2 -> t
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_expanded_token(),
                    Some(Token::Char('t', Category::Letter))
                );
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();

                // 1>2 -> f
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_expanded_token(),
                    Some(Token::Char('f', Category::Letter))
                );
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();

                // 1=2 -> f
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_expanded_token(),
                    Some(Token::Char('f', Category::Letter))
                );
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
            },
        );
    }

    #[test]
    fn it_allows_spaces_in_ifnum() {
        with_parser(&["\\ifnum 1       <      2      t\\fi%"], |parser| {
            parser.expand_conditional();
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('t', Category::Letter))
            );
            parser.expand_conditional();
        });
    }

    #[test]
    fn it_parses_count_variables_in_ifnum() {
        with_parser(
            &["\\ifnum \\count0 < \\count1 t\\else f\\fi%"],
            |parser| {
                parser.state.set_count(false, 0, 10);
                parser.state.set_count(false, 1, 20);

                // 10<20 -> t
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_expanded_token(),
                    Some(Token::Char('t', Category::Letter))
                );
                assert_eq!(parser.is_conditional_head(), true);
                parser.expand_conditional();
            },
        );
    }
}

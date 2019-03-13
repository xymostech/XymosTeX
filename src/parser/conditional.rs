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
    fn is_conditional_start(&mut self, token: &Token) -> bool {
        self.state.is_token_equal_to_prim(token, "iftrue")
            || self.state.is_token_equal_to_prim(token, "iffalse")
            || self.state.is_token_equal_to_prim(token, "ifnum")
    }

    pub fn is_conditional_head(&mut self) -> bool {
        match self.peek_unexpanded_token() {
            Some(token) => {
                self.is_conditional_start(&token)
                    || self.state.is_token_equal_to_prim(&token, "else")
                    || self.state.is_token_equal_to_prim(&token, "fi")
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
            if self.is_conditional_start(&token) {
                // If we see a conditional start while we're skipping, we need
                // to just skip to the end of that inner conditional before we
                // continue looking for the outer \fi.
                self.skip_to_fi();
            } else if self.state.is_token_equal_to_prim(&token, "fi") {
                break;
            } else if self.state.is_token_equal_to_prim(&token, "else") {
                ends_with_else = true;
                break;
            }
        }
        ends_with_else
    }

    // Skips tokens until a \fi is found.
    fn skip_to_fi(&mut self) {
        loop {
            let token = self.lex_unexpanded_token().unwrap();
            if self.is_conditional_start(&token) {
                // If we see a conditional start while we're skipping, we need
                // to just skip to the end of that inner conditional before we
                // continue looking for the outer \fi.
                self.skip_to_fi();
            } else if self.state.is_token_equal_to_prim(&token, "fi") {
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
            // When we encounter an \else, we know that we're in a 'true'
            // conditional because in a 'false' conditional, we always already
            // parse the \else token in skip_to_fi_or_else(). Thus, we just
            // need to skip tokens until we see a \fi.
            self.skip_to_fi();
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
    use crate::testing::with_parser;

    #[test]
    fn it_parses_single_body_iftrue() {
        with_parser(&["\\iftrue x\\fi%"], |parser| {
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('x', Category::Letter))
            );
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(parser.lex_unexpanded_token(), None);
        });
    }

    #[test]
    fn it_parses_iftrue_with_else() {
        with_parser(&["\\iftrue x\\else y\\fi%"], |parser| {
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('x', Category::Letter))
            );
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(parser.lex_unexpanded_token(), None);
        });
    }

    #[test]
    fn it_parses_single_body_iffalse() {
        with_parser(&["\\iffalse x\\fi%"], |parser| {
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(parser.lex_unexpanded_token(), None);
        });
    }

    #[test]
    fn it_parses_iffalse_with_else() {
        with_parser(&["\\iffalse x\\else y\\fi%"], |parser| {
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('y', Category::Letter))
            );
            assert_eq!(parser.is_conditional_head(), true);
            parser.expand_conditional();
            assert_eq!(parser.lex_unexpanded_token(), None);
        });
    }

    #[test]
    fn it_expand_macros_in_true_bodies_but_not_false_bodies() {
        with_parser(&["\\iftrue w\\a\\b\\fi%"], |parser| {
            parser.state.set_macro(
                false,
                &Token::ControlSequence("a".to_string()),
                &Rc::new(Macro::new(
                    vec![],
                    vec![
                        MacroListElem::Token(Token::Char(
                            'x',
                            Category::Letter,
                        )),
                        MacroListElem::Token(Token::ControlSequence(
                            "else".to_string(),
                        )),
                        MacroListElem::Token(Token::Char(
                            'y',
                            Category::Letter,
                        )),
                    ],
                )),
            );
            parser.state.set_macro(
                false,
                &Token::ControlSequence("b".to_string()),
                &Rc::new(Macro::new(
                    vec![],
                    vec![
                        MacroListElem::Token(Token::Char(
                            'z',
                            Category::Letter,
                        )),
                        MacroListElem::Token(Token::ControlSequence(
                            "fi".to_string(),
                        )),
                    ],
                )),
            );

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
        });
    }

    #[test]
    fn it_allows_conditional_primitives_to_be_let() {
        with_parser(
            &["\\iftruex a\\elsex b\\fix%", "\\iffalsex a\\elsex b\\fix%"],
            |parser| {
                parser.state.set_let(
                    false,
                    &Token::ControlSequence("iftruex".to_string()),
                    &Token::ControlSequence("iftrue".to_string()),
                );
                parser.state.set_let(
                    false,
                    &Token::ControlSequence("iffalsex".to_string()),
                    &Token::ControlSequence("iffalse".to_string()),
                );
                parser.state.set_let(
                    false,
                    &Token::ControlSequence("fix".to_string()),
                    &Token::ControlSequence("fi".to_string()),
                );
                parser.state.set_let(
                    false,
                    &Token::ControlSequence("elsex".to_string()),
                    &Token::ControlSequence("else".to_string()),
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
            },
        );
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

    #[test]
    fn it_handles_ifs_inside_of_ifs() {
        with_parser(
            &[
                "\\iftrue \\iftrue x\\fi \\fi%",
                "\\iffalse \\iftrue x\\fi \\fi%",
                "\\iftrue x\\else \\iftrue x\\fi \\fi%",
                "\\iffalse x\\else \\iftrue x\\fi \\fi%",
            ],
            |parser| {
                // true inside true
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_unexpanded_token(),
                    Some(Token::Char('x', Category::Letter))
                );
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert!(parser.is_conditional_head());
                parser.expand_conditional();

                // true inside false
                parser.expand_conditional();

                // true inside else of true
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_unexpanded_token(),
                    Some(Token::Char('x', Category::Letter))
                );
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert!(parser.is_conditional_head());

                // true inside else of false
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert_eq!(
                    parser.lex_unexpanded_token(),
                    Some(Token::Char('x', Category::Letter))
                );
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
                assert!(parser.is_conditional_head());
                parser.expand_conditional();
            },
        );
    }
}

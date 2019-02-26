use crate::category::Category;
use crate::parser::Parser;
use crate::token::Token;

#[derive(Debug, PartialEq, Eq)]
enum HorizontalListElem {
    Char(char),
}

impl<'a> Parser<'a> {
    fn replace_renamed_token(&mut self, maybe_token: Option<Token>) -> Option<Token> {
        match maybe_token {
            None => None,
            Some(ref token) => {
                if let Some(renamed) = self.state.get_renamed_token(token) {
                    Some(renamed)
                } else {
                    maybe_token
                }
            }
        }
    }

    fn parse_horizontal_list_elem(
        &mut self,
        group_level: &mut usize,
    ) -> Option<HorizontalListElem> {
        let expanded_token = self.peek_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);
        match expanded_renamed_token {
            None => None,
            Some(Token::Char(ch, cat)) => {
                self.lex_expanded_token();
                match cat {
                    Category::Letter => Some(HorizontalListElem::Char(ch)),
                    Category::Other => Some(HorizontalListElem::Char(ch)),
                    Category::Space => Some(HorizontalListElem::Char(' ')),
                    Category::BeginGroup => {
                        *group_level += 1;
                        self.state.push_state();
                        self.parse_horizontal_list_elem(group_level)
                    }
                    Category::EndGroup => {
                        if *group_level == 0 {
                            None
                        } else {
                            *group_level -= 1;
                            self.state.pop_state();
                            self.parse_horizontal_list_elem(group_level)
                        }
                    }
                    _ => panic!("unimplemented"),
                }
            }
            Some(Token::ControlSequence(seq)) => {
                if seq == "par" {
                    self.lex_expanded_token();
                    Some(HorizontalListElem::Char(' '))
                } else if self.is_assignment_head() {
                    self.parse_assignment();
                    self.parse_horizontal_list_elem(group_level)
                } else {
                    panic!("unimplemented!");
                }
            }
        }
    }

    fn parse_horizontal_list_to_elems(&mut self) -> Vec<HorizontalListElem> {
        let mut result = Vec::new();

        let mut group_level = 0;
        while let Some(elem) = self.parse_horizontal_list_elem(&mut group_level) {
            result.push(elem);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TeXState;

    fn assert_parses_to(lines: &[&str], expected_toks: &[HorizontalListElem]) {
        let state = TeXState::new();
        let mut parser = Parser::new(lines, &state);

        assert_eq!(parser.parse_horizontal_list_to_elems(), expected_toks);
    }

    #[test]
    fn it_parses_letters() {
        assert_parses_to(
            &["ab%"],
            &[HorizontalListElem::Char('a'), HorizontalListElem::Char('b')],
        );
    }

    #[test]
    fn it_parses_grouping() {
        assert_parses_to(
            &["a{b}c%"],
            &[
                HorizontalListElem::Char('a'),
                HorizontalListElem::Char('b'),
                HorizontalListElem::Char('c'),
            ],
        );
    }

    #[test]
    fn it_parses_assignments() {
        assert_parses_to(
            &["\\def\\a{b}%", "a\\a c%"],
            &[
                HorizontalListElem::Char('a'),
                HorizontalListElem::Char('b'),
                HorizontalListElem::Char('c'),
            ],
        );
    }

    #[test]
    fn it_handles_let_assigned_tokens() {
        assert_parses_to(&["\\let\\a=a%", "\\a%"], &[HorizontalListElem::Char('a')]);
    }

    #[test]
    fn it_handles_grouping() {
        assert_parses_to(
            &["\\def\\a{x}%", "{\\def\\a{y}\\a}%", "\\a"],
            &[HorizontalListElem::Char('y'), HorizontalListElem::Char('x')],
        );
    }
}

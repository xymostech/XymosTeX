use crate::category::Category;
use crate::glue::Glue;
use crate::parser::Parser;
use crate::token::Token;

#[derive(Debug, PartialEq)]
enum HorizontalListElem {
    Char(char),
    HSkip(Glue),
}

impl<'a> Parser<'a> {
    fn replace_renamed_token(
        &mut self,
        maybe_token: Option<Token>,
    ) -> Option<Token> {
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
            Some(ref tok) if self.state.is_token_equal_to_prim(tok, "par") => {
                self.lex_expanded_token();
                Some(HorizontalListElem::Char(' '))
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "hskip") =>
            {
                self.lex_expanded_token();
                let glue = self.parse_glue();
                Some(HorizontalListElem::HSkip(glue))
            }
            _ => {
                if self.is_assignment_head() {
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
        while let Some(elem) = self.parse_horizontal_list_elem(&mut group_level)
        {
            result.push(elem);
        }

        result
    }

    // For early testing, we're not going to worry about producing a box out of
    // the horizontal list, we'll only worry about the characters that are
    // produced by parsing a horizontal list. This pulls the characters we
    // parse out into a vec so external uses don't have to deal with
    // HorizontalListElems.
    pub fn parse_horizontal_list_to_chars(&mut self) -> Vec<char> {
        self.parse_horizontal_list_to_elems()
            .into_iter()
            .map(|elem| match elem {
                HorizontalListElem::Char(ch) => ch,
                HorizontalListElem::HSkip(_) => ' ',
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dimension::{Dimen, FilDimen, SpringDimen, Unit};
    use crate::testing::with_parser;

    fn assert_parses_to(lines: &[&str], expected_toks: &[HorizontalListElem]) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_horizontal_list_to_elems(), expected_toks);
        });
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
        assert_parses_to(
            &["\\let\\a=a%", "\\a%"],
            &[HorizontalListElem::Char('a')],
        );
    }

    #[test]
    fn it_handles_grouping() {
        assert_parses_to(
            &["\\def\\a{x}%", "{\\def\\a{y}\\a}%", "\\a"],
            &[HorizontalListElem::Char('y'), HorizontalListElem::Char('x')],
        );
    }

    #[test]
    fn it_parses_to_chars() {
        with_parser(&["bl ah%"], |parser| {
            assert_eq!(
                parser.parse_horizontal_list_to_chars(),
                vec!['b', 'l', ' ', 'a', 'h']
            );
        });
    }

    #[test]
    fn it_parses_hskip_tokens() {
        assert_parses_to(
            &["a\\hskip -3pt minus 2.3fil b%"],
            &[
                HorizontalListElem::Char('a'),
                HorizontalListElem::HSkip(Glue {
                    space: Dimen::from_unit(-3.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::FilDimen(FilDimen::Fil(2.3)),
                }),
                HorizontalListElem::Char('b'),
            ],
        );
    }
}

use crate::category::Category;
use crate::dimension::{Dimen, SpringDimen, Unit};
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::parser::Parser;
use crate::token::Token;

fn get_space_glue() -> Glue {
    Glue {
        space: Dimen::from_unit(3.33333, Unit::Point),
        stretch: SpringDimen::Dimen(Dimen::from_unit(1.66666, Unit::Point)),
        shrink: SpringDimen::Dimen(Dimen::from_unit(1.11111, Unit::Point)),
    }
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
            Some(Token::Char(ch, cat)) => match cat {
                Category::Letter => {
                    self.lex_expanded_token();
                    Some(HorizontalListElem::Char {
                        chr: ch,
                        font: self.state.get_current_font(),
                    })
                }
                Category::Other => {
                    self.lex_expanded_token();
                    Some(HorizontalListElem::Char {
                        chr: ch,
                        font: self.state.get_current_font(),
                    })
                }
                Category::Space => {
                    self.lex_expanded_token();
                    Some(HorizontalListElem::HSkip(get_space_glue()))
                }
                Category::BeginGroup => {
                    self.lex_expanded_token();
                    *group_level += 1;
                    self.state.push_state();
                    self.parse_horizontal_list_elem(group_level)
                }
                Category::EndGroup => {
                    if *group_level == 0 {
                        None
                    } else {
                        self.lex_expanded_token();
                        *group_level -= 1;
                        self.state.pop_state();
                        self.parse_horizontal_list_elem(group_level)
                    }
                }
                _ => panic!("unimplemented"),
            },
            Some(ref tok) if self.state.is_token_equal_to_prim(tok, "par") => {
                self.lex_expanded_token();
                self.parse_horizontal_list_elem(group_level)
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

    pub fn parse_horizontal_list(&mut self) -> Vec<HorizontalListElem> {
        let mut result = Vec::new();

        let mut group_level = 0;
        while let Some(elem) = self.parse_horizontal_list_elem(&mut group_level)
        {
            result.push(elem);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
    use crate::testing::with_parser;

    fn assert_parses_to(lines: &[&str], expected_toks: &[HorizontalListElem]) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_horizontal_list(), expected_toks);
        });
    }

    #[test]
    fn it_parses_letters() {
        assert_parses_to(
            &["ab%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }

    #[test]
    fn it_parses_grouping() {
        assert_parses_to(
            &["a{b}c%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }

    #[test]
    fn it_parses_assignments() {
        assert_parses_to(
            &["\\def\\a{b}%", "a\\a c%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }

    #[test]
    fn it_handles_let_assigned_tokens() {
        assert_parses_to(
            &["\\let\\a=a%", "\\a%"],
            &[HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            }],
        );
    }

    #[test]
    fn it_handles_grouping() {
        assert_parses_to(
            &["\\def\\a{x}%", "{\\def\\a{y}\\a}%", "\\a"],
            &[
                HorizontalListElem::Char {
                    chr: 'y',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'x',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }

    #[test]
    fn it_parses_space_to_glue() {
        assert_parses_to(
            &["a %"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::HSkip(get_space_glue()),
            ],
        );
    }

    #[test]
    fn it_ignores_par() {
        // NOTE(xymostech): This is only correct behavior in restricted
        // horizontal mode. There isn't currently a distinction between that
        // and normal horizontal mode here yet, so we'll just choose the easy
        // behavior.
        assert_parses_to(
            &["a%", "", "b%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }

    #[test]
    fn it_stops_parsing_at_mismatched_brace() {
        with_parser(&["a{b{c}d{e}f}g}%"], |parser| {
            let hlist = parser.parse_horizontal_list();
            assert_eq!(hlist.len(), 7);
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('}', Category::EndGroup))
            );
        });
    }

    #[test]
    fn it_parses_hskip_tokens() {
        assert_parses_to(
            &["a\\hskip -3pt minus 2.3fil b%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::HSkip(Glue {
                    space: Dimen::from_unit(-3.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::FilDimen(FilDimen::new(
                        FilKind::Fil,
                        2.3,
                    )),
                }),
                HorizontalListElem::Char {
                    chr: 'b',
                    font: "cmr10".to_string(),
                },
            ],
        );
    }
}

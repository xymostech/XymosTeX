use crate::category::Category;
use crate::list::VerticalListElem;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn parse_vertical_list_elem(
        &mut self,
        group_level: &mut usize,
        internal: bool,
    ) -> Option<VerticalListElem> {
        let expanded_token = self.peek_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);
        match expanded_renamed_token {
            None => None,
            Some(Token::Char(_, cat)) => match cat {
                Category::Space => {
                    self.lex_expanded_token();
                    self.parse_vertical_list_elem(group_level, internal)
                }
                Category::BeginGroup => {
                    self.lex_expanded_token();
                    *group_level += 1;
                    self.state.push_state();
                    self.parse_vertical_list_elem(group_level, internal)
                }
                Category::EndGroup => {
                    if *group_level == 0 {
                        None
                    } else {
                        self.lex_expanded_token();
                        *group_level -= 1;
                        self.state.pop_state();
                        self.parse_vertical_list_elem(group_level, internal)
                    }
                }
                _ => panic!("unimplemented"),
            },
            Some(ref tok) if self.state.is_token_equal_to_prim(tok, "end") => {
                if internal {
                    panic!(r"You can't use \end in internal vertical mode")
                }
                self.lex_expanded_token();
                None
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "vskip") =>
            {
                self.lex_expanded_token();
                let glue = self.parse_glue();
                Some(VerticalListElem::VSkip(glue))
            }
            _ => {
                if self.is_assignment_head() {
                    self.parse_assignment();
                    self.parse_vertical_list_elem(group_level, internal)
                } else {
                    panic!("unimplemented");
                }
            }
        }
    }

    fn parse_vertical_list(&mut self, internal: bool) -> Vec<VerticalListElem> {
        let mut result = Vec::new();

        let mut group_level = 0;
        while let Some(elem) =
            self.parse_vertical_list_elem(&mut group_level, internal)
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
    use crate::glue::Glue;
    use crate::testing::with_parser;

    fn assert_parses_to(lines: &[&str], expected_list: &[VerticalListElem]) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_vertical_list(true), expected_list);
        });
    }

    #[test]
    fn it_parses_vertical_glue() {
        assert_parses_to(
            &[r"\vskip 1pt%", r"\vskip 0pt plus1fil%"],
            &[
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
                VerticalListElem::VSkip(Glue {
                    space: Dimen::zero(),
                    stretch: SpringDimen::FilDimen(FilDimen::new(
                        FilKind::Fil,
                        1.0,
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
            ],
        );
    }

    #[test]
    fn it_ignores_spaces() {
        assert_parses_to(
            &[r"\vskip 1pt     \vskip 1pt%"],
            &[
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
            ],
        );
    }

    #[test]
    fn it_parses_assignments() {
        assert_parses_to(
            &[r"\def\a{\vskip 1pt}", r"\a\a%"],
            &[
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
            ],
        );
    }

    #[test]
    fn it_handles_grouping() {
        assert_parses_to(
            &[
                r"\def\a{\vskip 1pt}",
                r"{",
                r"\def\a{\vskip 2pt}",
                r"\a",
                r"}",
                r"\a",
            ],
            &[
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(2.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
            ],
        );
    }

    #[test]
    fn it_finishes_parsing_before_unmatched_close_group() {
        with_parser(&[r"{\vskip 1pt{{}\vskip 1pt}{}}}%"], |parser| {
            assert_eq!(
                parser.parse_vertical_list(true),
                &[
                    VerticalListElem::VSkip(Glue {
                        space: Dimen::from_unit(1.0, Unit::Point),
                        stretch: SpringDimen::Dimen(Dimen::zero()),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    }),
                    VerticalListElem::VSkip(Glue {
                        space: Dimen::from_unit(1.0, Unit::Point),
                        stretch: SpringDimen::Dimen(Dimen::zero()),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    }),
                ]
            );

            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('}', Category::EndGroup))
            );
        });
    }

    #[test]
    #[should_panic(expected = "You can't use \\end in internal vertical mode")]
    fn it_should_fail_with_end_in_internal_vertical_mode() {
        assert_parses_to(&[r"\vskip 0pt\end%"], &[]);
    }

    #[test]
    fn it_ends_non_internal_vertical_mode() {
        with_parser(&[r"\vskip 0pt\vskip 1pt\end a%"], |parser| {
            assert_eq!(
                parser.parse_vertical_list(false),
                &[
                    VerticalListElem::VSkip(Glue {
                        space: Dimen::from_unit(0.0, Unit::Point),
                        stretch: SpringDimen::Dimen(Dimen::zero()),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    }),
                    VerticalListElem::VSkip(Glue {
                        space: Dimen::from_unit(1.0, Unit::Point),
                        stretch: SpringDimen::Dimen(Dimen::zero()),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    }),
                ]
            );

            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }
}

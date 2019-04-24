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
        restricted: bool,
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
                    self.parse_horizontal_list_elem(group_level, restricted)
                }
                Category::EndGroup => {
                    if *group_level == 0 {
                        None
                    } else {
                        self.lex_expanded_token();
                        *group_level -= 1;
                        self.state.pop_state();
                        self.parse_horizontal_list_elem(group_level, restricted)
                    }
                }
                _ => panic!("unimplemented"),
            },
            Some(ref tok) if self.state.is_token_equal_to_prim(tok, "par") => {
                self.lex_expanded_token();

                if restricted {
                    self.parse_horizontal_list_elem(group_level, restricted)
                } else {
                    // In unrestricted horizontal mode, \par terminates the
                    // list parsing.
                    // TODO(xymostech): This also is supposed to do some extra
                    // work before finishing the list.
                    None
                }
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
                    self.parse_horizontal_list_elem(group_level, restricted)
                } else if self.is_box_head() {
                    let maybe_tex_box = self.parse_box();
                    if let Some(tex_box) = maybe_tex_box {
                        Some(HorizontalListElem::Box(tex_box))
                    } else {
                        self.parse_horizontal_list_elem(group_level, restricted)
                    }
                } else {
                    panic!("unimplemented!");
                }
            }
        }
    }

    pub fn parse_horizontal_list(
        &mut self,
        restricted: bool,
    ) -> Vec<HorizontalListElem> {
        let mut result = Vec::new();

        let mut group_level = 0;
        while let Some(elem) =
            self.parse_horizontal_list_elem(&mut group_level, restricted)
        {
            result.push(elem);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::{HorizontalBox, TeXBox};
    use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
    use crate::testing::with_parser;

    fn assert_parses_to_with_restricted(
        lines: &[&str],
        expected_toks: &[HorizontalListElem],
        restricted: bool,
    ) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_horizontal_list(restricted), expected_toks);
        });
    }

    fn assert_parses_to(lines: &[&str], expected_toks: &[HorizontalListElem]) {
        assert_parses_to_with_restricted(lines, expected_toks, true);
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
    fn it_stops_parsing_at_mismatched_brace() {
        with_parser(&["a{b{c}d{e}f}g}%"], |parser| {
            let hlist = parser.parse_horizontal_list(true);
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

    #[test]
    fn it_parses_explicit_box_elems() {
        with_parser(&[r"a\hbox{a\hskip 2pt plus1filg}b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

            let total_width = metrics.get_width('a')
                + metrics.get_width('g')
                + Dimen::from_unit(2.0, Unit::Point);

            assert_eq!(
                parser.parse_horizontal_list(true),
                &[
                    HorizontalListElem::Char {
                        chr: 'a',
                        font: "cmr10".to_string(),
                    },
                    HorizontalListElem::Box(TeXBox::HorizontalBox(
                        HorizontalBox {
                            width: total_width,
                            height: metrics.get_height('a'),
                            depth: metrics.get_depth('g'),

                            list: vec![
                                HorizontalListElem::Char {
                                    chr: 'a',
                                    font: "cmr10".to_string(),
                                },
                                HorizontalListElem::HSkip(Glue {
                                    space: Dimen::from_unit(2.0, Unit::Point),
                                    stretch: SpringDimen::FilDimen(
                                        FilDimen::new(FilKind::Fil, 1.0)
                                    ),
                                    shrink: SpringDimen::Dimen(Dimen::zero()),
                                }),
                                HorizontalListElem::Char {
                                    chr: 'g',
                                    font: "cmr10".to_string(),
                                },
                            ],
                            glue_set_ratio: None,
                        }
                    )),
                    HorizontalListElem::Char {
                        chr: 'b',
                        font: "cmr10".to_string(),
                    },
                ]
            );
        });
    }

    #[test]
    fn it_parses_box_register_elems() {
        with_parser(&[r"\setbox0=\hbox{a}%", r"\box0%"], |parser| {
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

            let list = parser.parse_horizontal_list(true);

            assert_eq!(list.len(), 1);
            if let HorizontalListElem::Box(ref tex_box) = list[0] {
                assert_eq!(tex_box.width(), &metrics.get_width('a'));
            } else {
                panic!("Element is not a box: {:?}", list[0]);
            }
        });
    }

    #[test]
    fn it_ignores_void_boxes() {
        assert_parses_to(
            &[r"a\box123b%"],
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
    fn it_leaves_horizontal_mode_when_seeing_par_in_unrestricted_mode() {
        // In unrestricted mode, \par ends the horizontal mode
        assert_parses_to_with_restricted(
            &[r"abc\par%"],
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
            false,
        );
    }

    #[test]
    fn it_ignores_par_in_restricted_mode() {
        // In restricted mode, \par does nothing
        assert_parses_to_with_restricted(
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
            true,
        );

        assert_parses_to_with_restricted(
            &[r"ab\par c%"],
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
            true,
        );
    }
}

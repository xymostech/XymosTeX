use crate::boxes::{HorizontalBox, TeXBox};
use crate::category::Category;
use crate::dimension::{Dimen, SpringDimen, Unit};
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::math_list::MathStyle;
use crate::parser::Parser;
use crate::token::Token;

fn get_space_glue() -> Glue {
    Glue {
        space: Dimen::from_unit(3.33333, Unit::Point),
        stretch: SpringDimen::Dimen(Dimen::from_unit(1.66666, Unit::Point)),
        shrink: SpringDimen::Dimen(Dimen::from_unit(1.11111, Unit::Point)),
    }
}

enum ElemResult {
    Elem(HorizontalListElem),
    Elems(Vec<HorizontalListElem>),
    Nothing,
}

impl<'a> Parser<'a> {
    /// Returns if the next token is the start of something that only makes
    /// sense in vertical mode.
    fn is_vertical_material_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["vskip", "end"])
    }

    fn parse_horizontal_list_elem(
        &mut self,
        group_level: &mut usize,
        restricted: bool,
    ) -> ElemResult {
        let expanded_token = self.peek_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);
        match expanded_renamed_token {
            None => ElemResult::Nothing,
            Some(Token::Char(ch, cat)) => match cat {
                Category::Letter => {
                    self.lex_expanded_token();
                    ElemResult::Elem(HorizontalListElem::Char {
                        chr: ch,
                        font: self.state.get_current_font(),
                    })
                }
                Category::Other => {
                    self.lex_expanded_token();
                    ElemResult::Elem(HorizontalListElem::Char {
                        chr: ch,
                        font: self.state.get_current_font(),
                    })
                }
                Category::Space => {
                    self.lex_expanded_token();
                    ElemResult::Elem(
                        HorizontalListElem::HSkip(get_space_glue()),
                    )
                }
                Category::BeginGroup => {
                    self.lex_expanded_token();
                    *group_level += 1;
                    self.state.push_state();
                    self.parse_horizontal_list_elem(group_level, restricted)
                }
                Category::EndGroup => {
                    if *group_level == 0 {
                        ElemResult::Nothing
                    } else {
                        self.lex_expanded_token();
                        *group_level -= 1;
                        self.state.pop_state();
                        self.parse_horizontal_list_elem(group_level, restricted)
                    }
                }
                Category::MathShift => {
                    self.lex_expanded_token();

                    let next_token = self.peek_unexpanded_token();
                    let is_next_token_math_shift = match next_token {
                        Some(Token::Char(_, Category::MathShift)) => true,
                        _ => false,
                    };

                    if !restricted && is_next_token_math_shift {
                        self.lex_unexpanded_token();

                        panic!("display math mode unimplemented!");
                    } else {
                        self.state.push_state();

                        let math_list = self.parse_math_list();
                        let horizontal_list = self
                            .convert_math_list_to_horizontal_list(
                                math_list,
                                MathStyle::TextStyle,
                            );

                        match self.lex_expanded_token() {
                            Some(Token::Char(_, Category::MathShift)) => {}
                            rest => {
                                panic!("Invalid end to math mode: {:?}", rest)
                            }
                        }

                        self.state.pop_state();

                        ElemResult::Elems(horizontal_list)
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
                    ElemResult::Nothing
                }
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "hskip") =>
            {
                self.lex_expanded_token();
                let glue = self.parse_glue();
                ElemResult::Elem(HorizontalListElem::HSkip(glue))
            }
            _ => {
                if self.is_assignment_head() {
                    self.parse_assignment();
                    self.parse_horizontal_list_elem(group_level, restricted)
                } else if self.is_box_head() {
                    let maybe_tex_box = self.parse_box();
                    if let Some(tex_box) = maybe_tex_box {
                        ElemResult::Elem(HorizontalListElem::Box(tex_box))
                    } else {
                        self.parse_horizontal_list_elem(group_level, restricted)
                    }
                } else if self.is_vertical_material_head() {
                    // If we see vertical mode material, we add a \par token to
                    // the input stream, continue and let that be parsed, after
                    // which we'll see the vertical mode material again.
                    self.add_upcoming_token(Token::ControlSequence(
                        "par".to_string(),
                    ));
                    self.parse_horizontal_list_elem(group_level, restricted)
                } else {
                    panic!("unimplemented!");
                }
            }
        }
    }

    pub fn parse_horizontal_list(
        &mut self,
        restricted: bool,
        indent: bool,
    ) -> Vec<HorizontalListElem> {
        let mut result = Vec::new();

        // Optionally add in indentation
        // TODO(xymostech): If I think about adding more flags for deciding the
        // "initial" state of the box, I need to think about whether this needs
        // needs to be better exposed, or if flags are the appropriate way to
        // control this.
        if indent {
            let mut hbox = HorizontalBox::empty();
            // TODO(xymostech): This should be \parindent, not a fixed 20pt.
            hbox.width = Dimen::from_unit(20.0, Unit::Point);
            let tex_box = TeXBox::HorizontalBox(hbox);
            result.push(HorizontalListElem::Box(tex_box));
        }

        let mut group_level = 0;

        loop {
            match self.parse_horizontal_list_elem(&mut group_level, restricted)
            {
                ElemResult::Nothing => break,
                ElemResult::Elem(elem) => result.push(elem),
                ElemResult::Elems(mut elems) => result.append(&mut elems),
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dimension::{FilDimen, FilKind};
    use crate::font::Font;
    use crate::math_code::MathCode;
    use crate::testing::with_parser;

    lazy_static! {
        static ref CMR10: Font = Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        };
    }

    fn assert_parses_to_with_restricted(
        lines: &[&str],
        expected_toks: &[HorizontalListElem],
        restricted: bool,
    ) {
        with_parser(lines, |parser| {
            assert_eq!(
                parser.parse_horizontal_list(restricted, false),
                expected_toks
            );
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: CMR10.clone(),
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
                font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'x',
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::HSkip(get_space_glue()),
            ],
        );
    }

    #[test]
    fn it_stops_parsing_at_mismatched_brace() {
        with_parser(&["a{b{c}d{e}f}g}%"], |parser| {
            let hlist = parser.parse_horizontal_list(true, false);
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
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
            ],
        );
    }

    #[test]
    fn it_parses_explicit_box_elems() {
        with_parser(&[r"a\hbox{a\hskip 2pt plus1filg}b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            let total_width = metrics.get_width('a')
                + metrics.get_width('g')
                + Dimen::from_unit(2.0, Unit::Point);

            assert_eq!(
                parser.parse_horizontal_list(true, false),
                &[
                    HorizontalListElem::Char {
                        chr: 'a',
                        font: CMR10.clone(),
                    },
                    HorizontalListElem::Box(TeXBox::HorizontalBox(
                        HorizontalBox {
                            width: total_width,
                            height: metrics.get_height('a'),
                            depth: metrics.get_depth('g'),

                            list: vec![
                                HorizontalListElem::Char {
                                    chr: 'a',
                                    font: CMR10.clone(),
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
                                    font: CMR10.clone(),
                                },
                            ],
                            glue_set_ratio: None,
                        }
                    )),
                    HorizontalListElem::Char {
                        chr: 'b',
                        font: CMR10.clone(),
                    },
                ]
            );
        });
    }

    #[test]
    fn it_parses_box_register_elems() {
        with_parser(&[r"\setbox0=\hbox{a}%", r"\box0%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            let list = parser.parse_horizontal_list(true, false);

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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: CMR10.clone(),
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
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
                },
            ],
            true,
        );

        assert_parses_to_with_restricted(
            &[r"ab\par c%"],
            &[
                HorizontalListElem::Char {
                    chr: 'a',
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'b',
                    font: CMR10.clone(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: CMR10.clone(),
                },
            ],
            true,
        );
    }

    #[test]
    fn it_adds_indentation() {
        with_parser(&[r"\setbox0=\hbox{}%", r"\wd0=20pt%", "a%"], |parser| {
            parser.parse_assignment();
            parser.parse_assignment();

            assert_eq!(
                parser.parse_horizontal_list(false, true),
                &[
                    HorizontalListElem::Box(parser.state.get_box(0).unwrap()),
                    HorizontalListElem::Char {
                        chr: 'a',
                        font: CMR10.clone(),
                    },
                ]
            );
        });
    }

    #[test]
    fn it_does_par_things_when_seeing_vertical_material() {
        // \par is defined normally, so we just end horizontal mode
        with_parser(&[r"a\end%"], |parser| {
            assert_eq!(
                parser.parse_horizontal_list(false, false),
                &[HorizontalListElem::Char {
                    chr: 'a',
                    font: CMR10.clone(),
                },]
            );
            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::ControlSequence("end".to_string()))
            );
        });

        // \par is defined specially, so we see its definition
        with_parser(
            &[r"\let\endgraf=\par%", r"\def\par{b\endgraf}%", r"a\end%"],
            |parser| {
                assert_eq!(
                    parser.parse_horizontal_list(false, false),
                    &[
                        HorizontalListElem::Char {
                            chr: 'a',
                            font: CMR10.clone(),
                        },
                        HorizontalListElem::Char {
                            chr: 'b',
                            font: CMR10.clone(),
                        },
                    ]
                );
                assert_eq!(
                    parser.lex_unexpanded_token(),
                    Some(Token::ControlSequence("end".to_string()))
                );
            },
        );
    }

    #[test]
    #[should_panic(expected = "unimplemented")]
    fn it_fails_parsing_mathchardefs() {
        with_parser(&[r"\hello%", r"\hello%"], |parser| {
            let tok = parser.lex_unexpanded_token().unwrap();
            parser.state.set_math_chardef(
                false,
                &tok,
                &MathCode::from_number(0x7161),
            );

            parser.parse_horizontal_list(false, false);
        });
    }

    #[test]
    fn it_parses_math_shifts() {
        with_parser(&[r"\hbox{a}\hbox{b}$ab$%"], |parser| {
            let box_a = parser.parse_box().unwrap();
            let box_b = parser.parse_box().unwrap();

            assert_eq!(
                parser.parse_horizontal_list(false, false),
                &[
                    HorizontalListElem::Box(box_a),
                    HorizontalListElem::Box(box_b),
                ]
            );
        });
    }

    #[test]
    fn it_adds_grouping_around_math_lists() {
        with_parser(
            &[
                r"\hbox{2}%",
                r"\count0=1 \number\count0 $\count0=2 \number\count0$\number\count0%",
            ],
            |parser| {
                let box_2 = parser.parse_box().unwrap();

                assert_eq!(
                    parser.parse_horizontal_list(false, false),
                    &[
                        HorizontalListElem::Char {
                            chr: '1',
                            font: CMR10.clone(),
                        },
                        HorizontalListElem::Box(box_2),
                        HorizontalListElem::Char {
                            chr: '1',
                            font: CMR10.clone(),
                        },
                    ]
                );
            },
        );
    }
}

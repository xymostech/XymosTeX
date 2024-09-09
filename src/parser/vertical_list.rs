use crate::category::Category;
use crate::dimension::{Dimen, Unit};
use crate::glue::Glue;
use crate::line_breaking::{
    break_horizontal_list_to_lines_with_params, LineBreakingParams,
};
use crate::list::{HorizontalListElem, VerticalListElem};
use crate::parser::assignment::SpecialVariables;
use crate::parser::Parser;
use crate::state::{DimenParameter, GlueParameter, IntegerParameter};
use crate::token::Token;

impl<'a> Parser<'a> {
    /// Handle generating an optionally indented horizontal mode box by
    /// entering horizontal mode and parsing the box there.
    fn handle_enter_horizontal_mode(
        &mut self,
        indent: bool,
    ) -> Vec<VerticalListElem> {
        // TODO(xymostech): Add \parskip glue before the box if the vertical list is empty.
        let mut list = self.parse_horizontal_list(false, indent);

        if let Some(HorizontalListElem::HSkip(_)) = list.last() {
            // If the last element of the list is a glue, we remove it.
            list.pop();
        }

        list.append(&mut vec![
            // TODO(xymostech): Add \nobreak before this and \break after this
            HorizontalListElem::HSkip(
                self.state.get_glue_parameter(&GlueParameter::ParFillSkip),
            ),
        ]);

        let maybe_boxes = break_horizontal_list_to_lines_with_params(
            &list,
            LineBreakingParams {
                hsize: self.state.get_dimen_parameter(&DimenParameter::HSize),
                tolerance: self
                    .state
                    .get_integer_parameter(&IntegerParameter::Tolerance),
                visual_incompatibility_demerits: self
                    .state
                    .get_integer_parameter(&IntegerParameter::AdjDemerits),
            },
            self.state,
        );

        if let Some(boxes) = maybe_boxes {
            boxes
                .into_iter()
                .map(|tex_box| VerticalListElem::Box {
                    tex_box: tex_box,
                    shift: Dimen::zero(),
                })
                .collect()
        } else {
            panic!("No valid line breaking found");
        }
    }

    /// Checks if a token is the start of something that only is valid in
    /// horizontal mode.
    fn is_horizontal_mode_head(&self, tok: &Token) -> bool {
        match tok {
            Token::Char(_, Category::Letter) => return true,
            Token::Char(_, Category::Other) => return true,
            Token::Char(_, Category::MathShift) => return true,
            _ => {}
        }

        if self.state.is_token_equal_to_prim(tok, "hskip")
            || self.state.is_token_equal_to_prim(tok, "char")
        {
            return true;
        }

        false
    }

    fn parse_vertical_list_elems(
        &mut self,
        group_level: &mut usize,
        prev_depth: &mut Dimen,
        internal: bool,
    ) -> Option<Vec<VerticalListElem>> {
        let expanded_token = self.peek_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);
        match expanded_renamed_token {
            None => {
                if internal {
                    None
                } else {
                    panic!(r"Emergency stop, EOF found before \end");
                }
            }
            Some(ref tok) if self.is_horizontal_mode_head(tok) => {
                Some(self.handle_enter_horizontal_mode(true))
            }
            Some(Token::Char(_, cat)) => match cat {
                Category::Space => {
                    self.lex_expanded_token();
                    self.parse_vertical_list_elems(
                        group_level,
                        prev_depth,
                        internal,
                    )
                }
                Category::BeginGroup => {
                    self.lex_expanded_token();
                    *group_level += 1;
                    self.state.push_state();
                    self.parse_vertical_list_elems(
                        group_level,
                        prev_depth,
                        internal,
                    )
                }
                Category::EndGroup => {
                    if *group_level == 0 {
                        if internal {
                            None
                        } else {
                            panic!("{}", "Too many }'s!");
                        }
                    } else {
                        self.lex_expanded_token();
                        *group_level -= 1;
                        self.state.pop_state();
                        self.parse_vertical_list_elems(
                            group_level,
                            prev_depth,
                            internal,
                        )
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
            Some(ref tok) if self.state.is_token_equal_to_prim(tok, "par") => {
                // \par is completely ignored
                self.lex_expanded_token();
                self.parse_vertical_list_elems(
                    group_level,
                    prev_depth,
                    internal,
                )
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "vskip") =>
            {
                self.lex_expanded_token();
                let glue = self.parse_glue();
                Some(vec![VerticalListElem::VSkip(glue)])
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "moveleft") =>
            {
                self.lex_expanded_token();
                let shift = self.parse_dimen();
                if let Some(tex_box) = self.parse_box() {
                    Some(vec![VerticalListElem::Box {
                        tex_box,
                        shift: shift * -1,
                    }])
                } else {
                    self.parse_vertical_list_elems(
                        group_level,
                        prev_depth,
                        internal,
                    )
                }
            }
            Some(ref tok)
                if self.state.is_token_equal_to_prim(tok, "moveright") =>
            {
                self.lex_expanded_token();
                let shift = self.parse_dimen();
                if let Some(tex_box) = self.parse_box() {
                    Some(vec![VerticalListElem::Box { tex_box, shift }])
                } else {
                    self.parse_vertical_list_elems(
                        group_level,
                        prev_depth,
                        internal,
                    )
                }
            }
            _ => {
                if self.is_assignment_head() {
                    self.parse_assignment(Some(SpecialVariables {
                        prev_depth: Some(prev_depth),
                    }));
                    self.parse_vertical_list_elems(
                        group_level,
                        prev_depth,
                        internal,
                    )
                } else if self.is_next_expanded_token_in_set_of_primitives(&[
                    "indent", "noindent",
                ]) {
                    let tok = self.lex_expanded_token().unwrap();
                    let indent =
                        self.state.is_token_equal_to_prim(&tok, "indent");
                    Some(self.handle_enter_horizontal_mode(indent))
                } else if self.is_box_head() {
                    let maybe_tex_box = self.parse_box();
                    if let Some(tex_box) = maybe_tex_box {
                        // TODO(xymostech): Insert interline glue here.
                        Some(vec![VerticalListElem::Box {
                            tex_box,
                            shift: Dimen::zero(),
                        }])
                    } else {
                        self.parse_vertical_list_elems(
                            group_level,
                            prev_depth,
                            internal,
                        )
                    }
                } else {
                    panic!("unimplemented");
                }
            }
        }
    }

    pub fn parse_vertical_list(
        &mut self,
        internal: bool,
    ) -> Vec<VerticalListElem> {
        let mut result = Vec::new();

        // The depth of the most recent box.
        let mut prev_depth = Dimen::from_unit(-1000.0, Unit::Point);

        // TODO(xymostech): Store these as \baselineskip, \lineskiplimit,
        // \lineskip, and \topskip parameters
        let baselineskip =
            Glue::from_dimen(Dimen::from_unit(12.0, Unit::Point));
        let lineskiplimit = Dimen::from_unit(0.0, Unit::Point);
        let lineskip = Glue::from_dimen(Dimen::from_unit(1.0, Unit::Point));
        let topskip = Glue::from_dimen(Dimen::from_unit(10.0, Unit::Point));

        let mut group_level = 0;
        while let Some(elems) = self.parse_vertical_list_elems(
            &mut group_level,
            &mut prev_depth,
            internal,
        ) {
            for elem in elems {
                // Handle box elements specially so we can add interline glue
                if let VerticalListElem::Box {
                    ref tex_box,
                    shift: _,
                } = elem
                {
                    // HACK(xymostech): \topskip should be handled in the outer
                    // place where we build pages, but we're doing it here since
                    // that doesn't exist yet.
                    if !internal && result.is_empty() {
                        let box_height = tex_box.height();
                        let total_skip =
                            topskip.clone() - Glue::from_dimen(*box_height);

                        if total_skip.space > Dimen::zero() {
                            result.push(VerticalListElem::VSkip(total_skip));
                        }
                    }

                    // If prev_depth is -1000pt, don't add interline glue
                    if prev_depth != Dimen::from_unit(-1000.0, Unit::Point) {
                        // Calculate how much interline glue we'd add if we just
                        // take into account baselineskip - prev_depth - box.height
                        let box_height = tex_box.height();
                        let total_skip = baselineskip.clone()
                            - Glue::from_dimen(*box_height + prev_depth);

                        // If the interline glue would be less than lineskiplimit,
                        // use lineskip instead.
                        let interline_glue = if total_skip.space < lineskiplimit
                        {
                            lineskip.clone()
                        } else {
                            total_skip
                        };

                        result.push(VerticalListElem::VSkip(interline_glue));
                    }

                    // Keep track of the depth of the most recent box
                    prev_depth = *tex_box.depth();
                }

                if !internal {
                    if let VerticalListElem::VSkip(_) = elem {
                        // Glue disappears at a page break.
                        if !result.is_empty() {
                            result.push(elem);
                        }
                    } else {
                        result.push(elem);
                    }
                } else {
                    result.push(elem);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use once_cell::sync::Lazy;

    use crate::boxes::{GlueSetRatio, GlueSetRatioKind, TeXBox, VerticalBox};
    use crate::dimension::{FilDimen, FilKind, SpringDimen};
    use crate::font::Font;
    use crate::testing::with_parser;

    static CMR10: Lazy<Font> = Lazy::new(|| Font {
        font_name: "cmr10".to_string(),
        scale: Dimen::from_unit(10.0, Unit::Point),
    });

    fn assert_parses_to(lines: &[&str], expected_list: &[VerticalListElem]) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_vertical_list(true), expected_list);
        });
    }

    fn assert_parses_to_non_internal(
        lines: &[&str],
        expected_list: &[VerticalListElem],
    ) {
        with_parser(lines, |parser| {
            assert_eq!(parser.parse_vertical_list(false), expected_list);
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
    #[should_panic(expected = r"You can't use \end in internal vertical mode")]
    fn it_should_fail_with_end_in_internal_vertical_mode() {
        assert_parses_to(&[r"\vskip 0pt\end%"], &[]);
    }

    #[test]
    fn it_skips_glue_at_the_beginning_of_non_internal_vertical_mode() {
        assert_parses_to_non_internal(&[r"\vskip 0pt\vskip 1pt\end%"], &[]);
    }

    #[test]
    fn it_ends_non_internal_vertical_mode() {
        with_parser(&[r"\hbox{}\end a%"], |parser| {
            let list = parser.parse_vertical_list(false);
            // \topskip + \hbox{}
            assert_eq!(list.len(), 2);

            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::Char('a', Category::Letter))
            );
        });
    }

    #[test]
    fn it_adds_topskip() {
        assert_parses_to_non_internal(
            &[r"\vbox{}\end%"],
            &[
                VerticalListElem::VSkip(Glue::from_dimen(Dimen::from_unit(
                    10.0,
                    Unit::Point,
                ))),
                VerticalListElem::Box {
                    tex_box: TeXBox::VerticalBox(VerticalBox {
                        height: Dimen::zero(),
                        depth: Dimen::zero(),
                        width: Dimen::zero(),
                        list: vec![],
                        glue_set_ratio: None,
                    }),
                    shift: Dimen::zero(),
                },
            ],
        );

        assert_parses_to_non_internal(
            &[r"\vbox to5pt{\vskip 0pt plus1pt}\end%"],
            &[
                VerticalListElem::VSkip(Glue::from_dimen(Dimen::from_unit(
                    5.0,
                    Unit::Point,
                ))),
                VerticalListElem::Box {
                    tex_box: TeXBox::VerticalBox(VerticalBox {
                        height: Dimen::from_unit(5.0, Unit::Point),
                        depth: Dimen::zero(),
                        width: Dimen::zero(),
                        list: vec![VerticalListElem::VSkip(Glue {
                            space: Dimen::zero(),
                            stretch: SpringDimen::Dimen(Dimen::from_unit(
                                1.0,
                                Unit::Point,
                            )),
                            shrink: SpringDimen::Dimen(Dimen::zero()),
                        })],
                        glue_set_ratio: Some(GlueSetRatio::from(
                            GlueSetRatioKind::Finite,
                            5.0,
                        )),
                    }),
                    shift: Dimen::zero(),
                },
            ],
        );

        assert_parses_to_non_internal(
            &[r"\vbox to15pt{\vskip 0pt plus1pt}\end%"],
            &[VerticalListElem::Box {
                tex_box: TeXBox::VerticalBox(VerticalBox {
                    height: Dimen::from_unit(15.0, Unit::Point),
                    depth: Dimen::zero(),
                    width: Dimen::zero(),
                    list: vec![VerticalListElem::VSkip(Glue {
                        space: Dimen::zero(),
                        stretch: SpringDimen::Dimen(Dimen::from_unit(
                            1.0,
                            Unit::Point,
                        )),
                        shrink: SpringDimen::Dimen(Dimen::zero()),
                    })],
                    glue_set_ratio: Some(GlueSetRatio::from(
                        GlueSetRatioKind::Finite,
                        15.0,
                    )),
                }),
                shift: Dimen::zero(),
            }],
        );
    }

    #[test]
    #[should_panic(expected = "Too many }'s")]
    fn it_should_fail_with_too_many_end_groups() {
        with_parser(&["{{}{{}}}}%"], |parser| {
            parser.parse_vertical_list(false);
        });
    }

    #[test]
    #[should_panic(expected = r"EOF found before \end")]
    fn it_should_fail_with_no_end() {
        with_parser(&[r"\vskip 0pt%"], |parser| {
            parser.parse_vertical_list(false);
        });
    }

    #[test]
    fn it_parses_box_elements() {
        with_parser(
            &[
                r"\setbox0=\hbox{a}%",
                r"\setbox1=\hbox{b}%",
                r"\setbox2=\hbox{b}%",
                r"\vskip 1pt%",
                r"\hbox{a}%",
                r"\vskip 2pt%",
                r"\box2",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                let box0 = parser.state.get_box(0).unwrap();
                let box1 = parser.state.get_box(1).unwrap();

                let interline_glue = Dimen::from_unit(12.0, Unit::Point)
                    - *box0.depth()
                    - *box1.height();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(1.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: box0,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(2.0, Unit::Point)
                        )),
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue
                        )),
                        VerticalListElem::Box {
                            tex_box: box1,
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_parses_hboxes_after_noindent() {
        with_parser(
            &[
                r"\hsize=1000pt%",
                r"\setbox0=\hbox to1000pt{a\hskip 0pt plus1fil}%",
                r"\setbox1=\hbox to1000pt{g\hskip 0pt plus1fil}%",
                r"\vskip 1pt%",
                r"\noindent a\par%",
                r"\vskip 2pt%",
                r"\noindent g\par%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                let box0 = parser.state.get_box(0).unwrap();
                let box1 = parser.state.get_box(1).unwrap();

                let interline_glue = Dimen::from_unit(12.0, Unit::Point)
                    - *box0.depth()
                    - *box1.height();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(1.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: box0,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(2.0, Unit::Point)
                        )),
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue
                        )),
                        VerticalListElem::Box {
                            tex_box: box1,
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_parses_hboxes_after_indent() {
        with_parser(
            &[
                r"\hsize=1000pt%",
                r"\setbox2=\hbox{}%",
                r"\wd2=20pt%",
                r"\setbox0=\hbox to1000pt{\copy2 a\hskip 0pt plus1fil}%",
                r"\setbox1=\hbox to1000pt{\copy2 g\hskip 0pt plus1fil}%",
                r"\vskip 1pt%",
                r"\indent a\par%",
                r"\vskip 2pt%",
                r"\indent g\par%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                let box0 = parser.state.get_box(0).unwrap();
                let box1 = parser.state.get_box(1).unwrap();

                let interline_glue = Dimen::from_unit(12.0, Unit::Point)
                    - *box0.depth()
                    - *box1.height();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(1.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: box0,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(2.0, Unit::Point)
                        )),
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue
                        )),
                        VerticalListElem::Box {
                            tex_box: box1,
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_enters_horizontal_mode_after_horizontal_material() {
        with_parser(
            &[
                r"\hsize=1000pt%",
                r"\setbox0=\hbox{}%",
                r"\wd0=20pt%",
                r"\def\line#1{\hbox to1000pt{#1\hskip0pt plus1fil}}%",
                r"\setbox1=\line{\copy0 a}%",
                r"\setbox2=\line{\copy0 @}%",
                r"\setbox3=\line{\copy0 $a$}%",
                r"\setbox4=\line{\copy0 \hskip1pt}%",
                r"a\par%",
                r"@\par%",
                r"$a$\par%",
                // The \hskip0pt is removed because it is a skip at the end of
                // the horizontal list
                r"\hskip 1pt\hskip 0pt\par%",
                r"\char 97\par%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                let box1 = parser.state.get_box(1).unwrap();
                let box2 = parser.state.get_box(2).unwrap();
                let box3 = parser.state.get_box(3).unwrap();
                let box4 = parser.state.get_box(4).unwrap();

                let interline_glue1 = Dimen::from_unit(12.0, Unit::Point)
                    - *box1.depth()
                    - *box2.height();
                let interline_glue2 = Dimen::from_unit(12.0, Unit::Point)
                    - *box2.depth()
                    - *box3.height();
                let interline_glue3 = Dimen::from_unit(12.0, Unit::Point)
                    - *box3.depth()
                    - *box4.height();
                let interline_glue4 = Dimen::from_unit(12.0, Unit::Point)
                    - *box4.depth()
                    - *box1.height();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::Box {
                            tex_box: box1.clone(),
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue1
                        )),
                        VerticalListElem::Box {
                            tex_box: box2,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue2
                        )),
                        VerticalListElem::Box {
                            tex_box: box3,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue3
                        )),
                        VerticalListElem::Box {
                            tex_box: box4,
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            interline_glue4
                        )),
                        VerticalListElem::Box {
                            tex_box: box1,
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_adds_interline_glue() {
        with_parser(
            &[
                r"\setbox0=\hbox{}%",
                r"\dp0=5pt%",
                r"\setbox1=\hbox{}%",
                r"\ht1=5pt%",
                r"\dp1=8pt%",
                r"\setbox2=\hbox{}%",
                r"\ht2=5pt%",
                r"\copy0%",
                r"\copy1%",
                r"\copy2%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(0).unwrap(),
                            shift: Dimen::zero()
                        },
                        // 12pt - 5pt - 5pt = 2pt of interline glue
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(2.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(1).unwrap(),
                            shift: Dimen::zero()
                        },
                        // 12pt - 8pt - 5pt = -1pt
                        // -1pt < 0pt (lineskiplimit), so we end up with
                        // lineskip (1pt) interline glue
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(1.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(2).unwrap(),
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_ignores_par() {
        with_parser(&[r"\vskip1pt", r"", r"\vskip1pt%"], |parser| {
            assert_eq!(
                parser.parse_vertical_list(true),
                &[
                    VerticalListElem::VSkip(Glue::from_dimen(
                        Dimen::from_unit(1.0, Unit::Point)
                    )),
                    VerticalListElem::VSkip(Glue::from_dimen(
                        Dimen::from_unit(1.0, Unit::Point)
                    )),
                ]
            );
        });
    }

    #[test]
    fn it_parses_moveleft_and_moveright_commands() {
        with_parser(
            &[
                r"\hbox{a}%",
                r"\vbox{b}%",
                r"\moveleft 2pt \hbox{a}\vskip 2pt\moveright 3pt \vbox{b}%",
            ],
            |parser| {
                let abox = parser.parse_box().unwrap();
                let bbox = parser.parse_box().unwrap();

                let metrics =
                    parser.state.get_metrics_for_font(&CMR10).unwrap();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::Box {
                            tex_box: abox,
                            shift: Dimen::from_unit(-2.0, Unit::Point),
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(2.0, Unit::Point)
                        )),
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(12.0, Unit::Point)
                                - metrics.get_height('b')
                        )),
                        VerticalListElem::Box {
                            tex_box: bbox,
                            shift: Dimen::from_unit(3.0, Unit::Point),
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_ignores_empty_boxes_in_raise_and_lower() {
        with_parser(
            &[
                r"\hbox{a}%",
                r"\hbox{a}\moveleft 2pt \box10\moveright 2pt \box11%",
            ],
            |parser| {
                let abox = parser.parse_box().unwrap();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[VerticalListElem::Box {
                        tex_box: abox,
                        shift: Dimen::zero(),
                    },]
                );
            },
        );
    }

    #[test]
    fn it_allows_prev_depth_assignments() {
        with_parser(&[r"\prevdepth=2pt%", r"\vskip 2pt%"], |parser| {
            parser.parse_vertical_list(true);
        });
    }

    #[test]
    fn it_uses_updated_prev_depth_values() {
        with_parser(
            &[
                r"\setbox0=\hbox{}%",
                r"\dp0=5pt%",
                r"\setbox1=\hbox{}%",
                r"\ht1=5pt%",
                r"\dp1=8pt%",
                r"\setbox2=\hbox{}%",
                r"\ht2=5pt%",
                r"\copy0%",
                r"\prevdepth=3pt%",
                r"\copy1%",
                r"\prevdepth=-1000pt%",
                r"\copy2%",
            ],
            |parser| {
                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(0).unwrap(),
                            shift: Dimen::zero()
                        },
                        // prevdepth=3pt instead of the original 5pt
                        // 12pt - 3pt - 5pt = 4pt of interline glue
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(4.0, Unit::Point)
                        )),
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(1).unwrap(),
                            shift: Dimen::zero()
                        },
                        // no interline glue because prevdepth was set to
                        // -1000pt
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(2).unwrap(),
                            shift: Dimen::zero()
                        },
                    ]
                );
            },
        );
    }

    #[test]
    fn it_splits_horizontal_lists_into_lines() {
        with_parser(
            &[
                r"\setbox1=\hbox to50pt{abc def ghi}%",
                r"\setbox2=\hbox to50pt{jkl mno pqr}%",
                r"\setbox3=\hbox to50pt{stu vwx yz\hskip 0pt plus1fil}%",
                r"\hsize=50pt%",
                r"\noindent abc def ghi jkl mno pqr stu vwx yz%",
            ],
            |parser| {
                let metrics =
                    parser.state.get_metrics_for_font(&CMR10).unwrap();

                assert_eq!(
                    parser.parse_vertical_list(true),
                    &[
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(1).unwrap(),
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(12.0, Unit::Point)
                                - metrics.get_depth('g')
                                - metrics.get_height('k'),
                        )),
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(2).unwrap(),
                            shift: Dimen::zero()
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(
                            Dimen::from_unit(12.0, Unit::Point)
                                - metrics.get_depth('j')
                                - metrics.get_height('t'),
                        )),
                        VerticalListElem::Box {
                            tex_box: parser.state.get_box(3).unwrap(),
                            shift: Dimen::zero()
                        },
                    ],
                );
            },
        );
    }
}

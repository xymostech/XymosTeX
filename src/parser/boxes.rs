use crate::boxes::{
    get_set_dimen_and_ratio, BoxLayout, HorizontalBox, TeXBox, VerticalBox,
};
use crate::category::Category;
use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::list::HorizontalListElem;
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    pub fn add_to_natural_layout_horizontal_box(
        &mut self,
        mut hbox: HorizontalBox,
        elem: HorizontalListElem,
    ) -> HorizontalBox {
        if hbox.glue_set_ratio.is_some() {
            panic!("Cannot add to an hbox with non-empty glue set ratio");
        }

        let (elem_height, elem_depth, elem_width) = elem.get_size(self.state);

        if elem_height > hbox.height {
            hbox.height = elem_height;
        }
        if elem_depth > hbox.depth {
            hbox.depth = elem_depth;
        }

        hbox.width = hbox.width + elem_width.space;
        hbox.list.push(elem);

        hbox
    }

    fn parse_horizontal_box(
        &mut self,
        layout: &BoxLayout,
        restricted: bool,
        indent: bool,
    ) -> HorizontalBox {
        let list = self.parse_horizontal_list(restricted, indent);
        HorizontalBox::create_from_horizontal_list_with_layout(
            list, layout, self.state,
        )
    }

    /// Provides an easy way for external consumers of boxes to parse a
    /// specific type of horizontal box, so they don't have to be concerned
    /// with BoxLayout or TeXBox vs HorizontalBox.
    pub fn parse_unrestricted_horizontal_box(
        &mut self,
        indent: bool,
    ) -> TeXBox {
        let hbox =
            self.parse_horizontal_box(&BoxLayout::Natural, false, indent);
        TeXBox::HorizontalBox(hbox)
    }

    fn parse_vertical_box(
        &mut self,
        layout: &BoxLayout,
        internal: bool,
    ) -> VerticalBox {
        // Parse the actual list of elements
        let list = self.parse_vertical_list(internal);

        // Keep track of the total height of the elements
        let mut height = Glue::zero();
        // Keep track of the depth of the most recently seen element. This will
        // end up 0 for all elements except for boxes
        let mut prev_depth = Dimen::zero();
        // Keep track of the maximum element width
        let mut width = Dimen::zero();

        for elem in &list {
            let (elem_height, elem_depth, elem_width) = elem.get_size();

            // Add up the height of the elements, plus the depths for all but
            // the last element. get_size() returns a Glue for the height, but
            // the depths are just dimens, so we convert it.
            height = height + Glue::from_dimen(prev_depth) + elem_height;

            // Keep track of the depth of the most recent element
            prev_depth = elem_depth;

            // Find the maximum width of all the elements
            if elem_width > width {
                width = elem_width;
            }
        }

        // Figure out the true height and set ratio
        let (set_height, glue_set) = get_set_dimen_and_ratio(height, layout);

        VerticalBox {
            height: set_height,
            depth: prev_depth,
            width,

            list,
            glue_set_ratio: glue_set,
        }
    }

    fn parse_box_specification(&mut self) -> BoxLayout {
        if self.parse_optional_keyword_expanded("to") {
            let dimen = self.parse_dimen();
            self.parse_filler_expanded();
            BoxLayout::Fixed(dimen)
        } else if self.parse_optional_keyword_expanded("spread") {
            let dimen = self.parse_dimen();
            self.parse_filler_expanded();
            BoxLayout::Spread(dimen)
        } else {
            self.parse_filler_expanded();
            BoxLayout::Natural
        }
    }

    pub fn is_box_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "hbox", "vbox", "box", "copy",
        ])
    }

    pub fn parse_box(&mut self) -> Option<TeXBox> {
        let head = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&head, "hbox") {
            let layout = self.parse_box_specification();

            // We expect a { after the box specification
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::BeginGroup)) => (),
                _ => panic!("{}", "Expected { when parsing box"),
            }

            self.state.push_state();

            let hbox = self.parse_horizontal_box(&layout, true, false);

            self.state.pop_state();

            // And there should always be a } after the horizontal list
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::EndGroup)) => (),
                _ => panic!("{}", "Expected } when parsing box"),
            }

            Some(TeXBox::HorizontalBox(hbox))
        } else if self.state.is_token_equal_to_prim(&head, "vbox") {
            let layout = self.parse_box_specification();

            // We expect a { after the box specification
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::BeginGroup)) => (),
                _ => panic!("{}", "Expected { when parsing box"),
            }

            self.state.push_state();

            let vbox = self.parse_vertical_box(&layout, true);

            self.state.pop_state();

            // And there should always be a } after the horizontal list
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::EndGroup)) => (),
                _ => panic!("{}", "Expected } when parsing box"),
            }

            Some(TeXBox::VerticalBox(vbox))
        } else if self.state.is_token_equal_to_prim(&head, "box") {
            let box_index = self.parse_8bit_number();
            self.state.get_box(box_index)
        } else if self.state.is_token_equal_to_prim(&head, "copy") {
            let box_index = self.parse_8bit_number();
            self.state.get_box_copy(box_index)
        } else {
            panic!("unimplemented");
        }
    }

    // Used for early testing, when we want to output test the output of
    // parsing an entire box.
    pub fn parse_outer_vertical_box(&mut self) -> VerticalBox {
        self.parse_vertical_box(&BoxLayout::Natural, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use once_cell::sync::Lazy;

    use crate::boxes::{GlueSetRatio, GlueSetRatioKind};
    use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
    use crate::font::Font;
    use crate::testing::with_parser;

    static CMR10: Lazy<Font> = Lazy::new(|| Font {
        font_name: "cmr10".to_string(),
        scale: Dimen::from_unit(10.0, Unit::Point),
    });

    #[test]
    fn it_parses_boxes_with_characters() {
        with_parser(&["gb%"], |parser| {
            let hbox =
                parser.parse_horizontal_box(&BoxLayout::Natural, true, false);

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            assert_eq!(hbox.height, metrics.get_height('b'));
            assert_eq!(hbox.depth, metrics.get_depth('g'));
            assert_eq!(
                hbox.width,
                metrics.get_width('g') + metrics.get_width('b')
            );
        });
    }

    #[test]
    fn it_parses_boxes_with_glue() {
        with_parser(&["\\hskip 1pt \\hskip 2pt plus 1fil%"], |parser| {
            let hbox =
                parser.parse_horizontal_box(&BoxLayout::Natural, true, false);

            assert_eq!(hbox.height, Dimen::zero());
            assert_eq!(hbox.depth, Dimen::zero());
            assert_eq!(hbox.width, Dimen::from_unit(3.0, Unit::Point));
        });
    }

    #[test]
    fn it_parses_boxes_with_glue_and_characters() {
        with_parser(&["b\\hskip 2pt g%"], |parser| {
            let hbox =
                parser.parse_horizontal_box(&BoxLayout::Natural, true, false);

            assert_eq!(hbox.list.len(), 3);

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            assert_eq!(hbox.height, metrics.get_height('b'));
            assert_eq!(hbox.depth, metrics.get_depth('g'));
            assert_eq!(
                hbox.width,
                metrics.get_width('g')
                    + metrics.get_width('b')
                    + Dimen::from_unit(2.0, Unit::Point)
            );
        });
    }

    #[test]
    fn it_stretches_boxes_with_finite_glue_to_a_fixed_width() {
        with_parser(&["a\\hskip 0pt plus1pt b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 5.0))
            );
        });
    }

    #[test]
    fn it_stretches_boxes_with_infinite_glue_to_a_fixed_width() {
        // Fil
        with_parser(&["a\\hskip 0pt plus1fil b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 5.0))
            );
        });

        // Fill
        with_parser(&["a\\hskip 0pt plus1fill b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fill, 5.0))
            );
        });

        // Filll
        with_parser(&["a\\hskip 0pt plus1filll b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Filll, 5.0))
            );
        });
    }

    #[test]
    fn it_combines_glue_when_setting() {
        with_parser(
            &["a\\hskip 0pt plus1pt\\hskip 0pt plus2pt b%"],
            |parser| {
                let metrics =
                    parser.state.get_metrics_for_font(&CMR10).unwrap();
                let fixed_width = metrics.get_width('a')
                    + metrics.get_width('b')
                    + Dimen::from_unit(6.0, Unit::Point);

                let hbox = parser.parse_horizontal_box(
                    &BoxLayout::Fixed(fixed_width),
                    true,
                    false,
                );

                assert_eq!(hbox.width, fixed_width);
                assert_eq!(
                    hbox.glue_set_ratio,
                    Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 2.0))
                );
            },
        );
    }

    #[test]
    fn it_shrinks_boxes_with_finite_glue_when_setting_to_fixed_width() {
        with_parser(&["a\\hskip 0pt minus2ptb%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let fixed_width = metrics.get_width('a') + metrics.get_width('b')
                - Dimen::from_unit(1.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5))
            );
        });
    }

    #[test]
    fn it_shrinks_boxes_with_infinite_glue_when_setting_to_fixed_width() {
        with_parser(&["a\\hskip 0pt minus1fil b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let fixed_width = metrics.get_width('a') + metrics.get_width('b')
                - Dimen::from_unit(4.0, Unit::Point);

            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Fixed(fixed_width),
                true,
                false,
            );

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -4.0))
            );
        });
    }

    #[test]
    fn it_stretches_boxes_with_finite_glue_when_spread() {
        with_parser(&["a\\hskip 0pt plus3pt b%"], |parser| {
            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Spread(Dimen::from_unit(6.0, Unit::Point)),
                true,
                false,
            );

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(6.0, Unit::Point);

            assert_eq!(hbox.width, expected_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 2.0))
            );
        });
    }

    #[test]
    fn it_stretches_boxes_with_infinite_glue_when_spread() {
        with_parser(&["a\\hskip 0pt plus1fill b%"], |parser| {
            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Spread(Dimen::from_unit(6.0, Unit::Point)),
                true,
                false,
            );

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(6.0, Unit::Point);

            assert_eq!(hbox.width, expected_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fill, 6.0))
            );
        });
    }

    #[test]
    fn it_shrinks_boxes_with_finite_glue_when_spread() {
        with_parser(&["a\\hskip 0pt minus2pt b%"], |parser| {
            let hbox = parser.parse_horizontal_box(
                &BoxLayout::Spread(Dimen::from_unit(-1.0, Unit::Point)),
                true,
                false,
            );

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('b')
                - Dimen::from_unit(1.0, Unit::Point);

            assert_eq!(hbox.width, expected_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5))
            );
        });
    }

    #[test]
    fn it_parses_horizontal_boxes_with_natural_width() {
        with_parser(&["\\hbox{abc}%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('b')
                + metrics.get_width('c');

            assert!(parser.is_box_head());
            let hbox = parser.parse_box().unwrap();
            if let TeXBox::HorizontalBox(hbox) = hbox {
                assert_eq!(hbox.list.len(), 3);
                assert_eq!(hbox.glue_set_ratio, None);
                assert_eq!(hbox.width, expected_width);
            } else {
                panic!("Found vbox!");
            }
        });
    }

    #[test]
    fn it_parses_horizontal_boxes_with_fixed_width() {
        with_parser(&["\\hbox to20pt{a\\hskip 0pt plus1filc}%"], |parser| {
            assert!(parser.is_box_head());
            let hbox = parser.parse_box().unwrap();
            if let TeXBox::HorizontalBox(hbox) = hbox {
                assert_eq!(hbox.list.len(), 3);
                assert_eq!(hbox.width, Dimen::from_unit(20.0, Unit::Point));
            } else {
                panic!("Found vbox!");
            }
        });
    }

    #[test]
    fn it_parses_horizontal_boxes_with_spread_width() {
        with_parser(&["\\hbox spread5pt{a\\hskip 0pt plus1filc}%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('c')
                + Dimen::from_unit(5.0, Unit::Point);

            assert!(parser.is_box_head());
            let hbox = parser.parse_box().unwrap();
            if let TeXBox::HorizontalBox(hbox) = hbox {
                assert_eq!(hbox.list.len(), 3);
                assert_eq!(hbox.width, expected_width);
            } else {
                panic!("Found vbox!");
            }
        });
    }

    #[test]
    fn it_parses_boxes_from_box_registers() {
        with_parser(&[r"\setbox0=\hbox{a}%", r"\box0", r"\box0"], |parser| {
            parser.parse_assignment(None);

            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            assert!(parser.is_box_head());
            let parsed_box = parser.parse_box().unwrap();
            assert_eq!(parsed_box.width(), &metrics.get_width('a'));

            assert!(parser.is_box_head());
            assert_eq!(parser.parse_box(), None);
        });
    }

    #[test]
    fn it_parses_copied_boxes_from_box_registers() {
        with_parser(&[r"\setbox0=\hbox{a}%", r"\copy0", r"\box0"], |parser| {
            parser.parse_assignment(None);

            assert!(parser.is_box_head());
            let copied_box = parser.parse_box().unwrap();
            assert_eq!(copied_box, parser.state.get_box_copy(0).unwrap());

            assert!(parser.is_box_head());
            let parsed_box = parser.parse_box().unwrap();
            assert_eq!(parsed_box, copied_box);
        });
    }

    #[test]
    fn it_parses_vertical_lists() {
        with_parser(&[r"aby%", r"\vskip 2pt%", r"g%"], |parser| {
            let metrics = parser.state.get_metrics_for_font(&CMR10).unwrap();

            let vbox = parser.parse_vertical_box(&BoxLayout::Natural, true);

            // Sanity check the number of elements to make sure something
            // didn't go horribly wrong.
            assert_eq!(vbox.list.len(), 4);

            // The height will be the height of the first box + the 12pt of
            // interline glue + the 2pt glue
            let expected_height = metrics.get_height('b')
                + Dimen::from_unit(12.0, Unit::Point)
                + Dimen::from_unit(2.0, Unit::Point);
            assert_eq!(vbox.height, expected_height);

            // The depth will just be the depth of the second box.
            assert_eq!(vbox.depth, metrics.get_depth('g'));

            // The width will be the width of the first box, which is indented
            // and contains a, b, and y.
            let expected_width = Dimen::from_unit(20.0, Unit::Point)
                + metrics.get_width('a')
                + metrics.get_width('b')
                + metrics.get_width('y');
            assert_eq!(vbox.width, expected_width);
        });
    }

    #[test]
    fn it_sets_fixed_glue_in_vertical_lists() {
        // This example is taken from the TeXbook, exercise 12.12
        with_parser(
            &[
                r"\setbox1=\hbox{}%",
                r"\wd1=1pt \ht1=1pt \dp1=1pt%",
                r"\setbox2=\hbox{}%",
                r"\wd2=2pt \ht2=2pt \dp2=2pt%",
                r"\vskip0pt plus1fil minus1fil%",
                r"\box1%",
                // This is added to add to account for the
                // \baselineskip=9pt minus3fil
                r"\vskip-3pt minus3fil%",
                r"\box2%",
                r"\vskip0pt plus1fil minus1fil%",
            ],
            |parser| {
                let vbox = parser.parse_vertical_box(
                    &BoxLayout::Fixed(Dimen::from_unit(4.0, Unit::Point)),
                    true,
                );

                // Sanity check the number of elements to make sure something
                // didn't go horribly wrong.
                assert_eq!(vbox.list.len(), 6);

                // Since we specified a fixed layout, this is just the fixed amount
                assert_eq!(vbox.height, Dimen::from_unit(4.0, Unit::Point));
                // The last element is not a box, so the overall depth is 0
                assert_eq!(vbox.depth, Dimen::zero());
                // In the exercise, the first box is moved over, so this ends up
                // being 1. Since we aren't moving it over, this is the max of the
                // two boxes, which is 2.
                assert_eq!(vbox.width, Dimen::from_unit(2.0, Unit::Point));
                // The glue set ratio ends up being -8/5 pt/fil
                assert_eq!(
                    vbox.glue_set_ratio,
                    Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -8.0 / 5.0))
                );
            },
        );
    }

    #[test]
    fn it_parses_vbox() {
        with_parser(
            &[
                r"\setbox0=\hbox{x}%",
                r"\vbox to 20pt{\box0 a\vskip1pt plus1fil g}%",
            ],
            |parser| {
                let metrics =
                    parser.state.get_metrics_for_font(&CMR10).unwrap();

                parser.parse_assignment(None);
                let vbox = parser.parse_box().unwrap();

                assert_eq!(*vbox.height(), Dimen::from_unit(20.0, Unit::Point));
                assert_eq!(*vbox.depth(), metrics.get_depth('g'));
            },
        );
    }

    #[test]
    fn it_parses_state_group_around_box_definitions() {
        with_parser(
            &[
                r"\setbox0=\hbox{%",
                r"\count0=1 %",
                r"\number\count0%",
                r"\hbox{\number\count0\count0=2 \number\count0}%",
                r"\number\count0%",
                r"\vbox{\number\count0\count0=2 \number\count0}%",
                r"\number\count0%",
                r"}%",
                r"\setbox1=\hbox{1\hbox{12}1\vbox{12}1}%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                assert_eq!(parser.state.get_box(0), parser.state.get_box(1),);
            },
        );
    }

    #[test]
    fn it_rounds_glue_set_ratio_to_the_nearest_65536th() {
        with_parser(
            &[
                r"\setbox0=\hbox to1sp{\hskip 0pt plus 2fil}%",
                r"\setbox1=\hbox to500sp{\hskip 0pt plus 1000fil}%",
                r"\setbox2=\hbox to500sp{\hskip 0pt plus 1001fil}%",
                r"\setbox3=\hbox to1500sp{\hskip 0pt plus 1000fil}%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);

                assert_eq!(
                    parser.state.get_box(0),
                    Some(TeXBox::HorizontalBox(HorizontalBox {
                        height: Dimen::zero(),
                        depth: Dimen::zero(),
                        width: Dimen::from_unit(1.0, Unit::ScaledPoint),

                        list: vec![HorizontalListElem::HSkip(Glue {
                            space: Dimen::zero(),
                            stretch: SpringDimen::FilDimen(FilDimen::new(
                                FilKind::Fil,
                                2.0
                            )),
                            shrink: SpringDimen::Dimen(Dimen::zero()),
                        }),],
                        glue_set_ratio: Some(GlueSetRatio::from(
                            GlueSetRatioKind::Fil,
                            1.0 / 65536.0
                        )),
                    }))
                );

                assert_eq!(
                    parser.state.get_box(1),
                    Some(TeXBox::HorizontalBox(HorizontalBox {
                        height: Dimen::zero(),
                        depth: Dimen::zero(),
                        width: Dimen::from_unit(500.0, Unit::ScaledPoint),

                        list: vec![HorizontalListElem::HSkip(Glue {
                            space: Dimen::zero(),
                            stretch: SpringDimen::FilDimen(FilDimen::new(
                                FilKind::Fil,
                                1000.0
                            )),
                            shrink: SpringDimen::Dimen(Dimen::zero()),
                        }),],
                        glue_set_ratio: Some(GlueSetRatio::from(
                            GlueSetRatioKind::Fil,
                            1.0 / 65536.0
                        )),
                    }))
                );

                assert_eq!(
                    parser.state.get_box(2),
                    Some(TeXBox::HorizontalBox(HorizontalBox {
                        height: Dimen::zero(),
                        depth: Dimen::zero(),
                        width: Dimen::from_unit(500.0, Unit::ScaledPoint),

                        list: vec![HorizontalListElem::HSkip(Glue {
                            space: Dimen::zero(),
                            stretch: SpringDimen::FilDimen(FilDimen::new(
                                FilKind::Fil,
                                1001.0
                            )),
                            shrink: SpringDimen::Dimen(Dimen::zero()),
                        }),],
                        glue_set_ratio: Some(GlueSetRatio::from(
                            GlueSetRatioKind::Fil,
                            0.0
                        )),
                    }))
                );

                assert_eq!(
                    parser.state.get_box(3),
                    Some(TeXBox::HorizontalBox(HorizontalBox {
                        height: Dimen::zero(),
                        depth: Dimen::zero(),
                        width: Dimen::from_unit(1500.0, Unit::ScaledPoint),

                        list: vec![HorizontalListElem::HSkip(Glue {
                            space: Dimen::zero(),
                            stretch: SpringDimen::FilDimen(FilDimen::new(
                                FilKind::Fil,
                                1000.0
                            )),
                            shrink: SpringDimen::Dimen(Dimen::zero()),
                        }),],
                        glue_set_ratio: Some(GlueSetRatio::from(
                            GlueSetRatioKind::Fil,
                            2.0 / 65536.0
                        )),
                    }))
                );
            },
        );
    }
}

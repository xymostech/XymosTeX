use crate::boxes::{GlueSetRatio, GlueSetRatioKind, HorizontalBox, TeXBox};
use crate::category::Category;
use crate::dimension::{Dimen, SpringDimen};
use crate::glue::Glue;
use crate::parser::Parser;
use crate::token::Token;

pub enum BoxLayout {
    NaturalWidth,
    FixedWidth(Dimen),
    Spread(Dimen),
}

// Given the amount of stretch/shrink needed to set a given box and the amount
// of stretch/shrink available, figure out the glue set ratio.
fn set_glue(
    stretch_needed: &Dimen,
    stretch_available: &SpringDimen,
) -> GlueSetRatio {
    match stretch_available {
        SpringDimen::Dimen(stretch_dimen) => GlueSetRatio::from(
            GlueSetRatioKind::Finite,
            // TODO(xymostech): Ensure this isn't <-1.0
            stretch_needed / stretch_dimen,
        ),

        SpringDimen::FilDimen(stretch_fil_dimen) => GlueSetRatio::from(
            GlueSetRatioKind::from_fil_kind(&stretch_fil_dimen.0),
            stretch_needed / stretch_fil_dimen,
        ),
    }
}

impl<'a> Parser<'a> {
    pub fn parse_horizontal_box(
        &mut self,
        layout: &BoxLayout,
    ) -> HorizontalBox {
        let list = self.parse_horizontal_list();

        let mut height = Dimen::zero();
        let mut depth = Dimen::zero();
        let mut width = Glue::zero();

        for elem in &list {
            let (elem_height, elem_depth, elem_width) =
                elem.get_size(self.state);

            if elem_height > height {
                height = elem_height;
            }
            if elem_depth > depth {
                depth = elem_depth;
            }
            width = width + elem_width;
        }

        let (set_width, set_ratio) = match layout {
            &BoxLayout::NaturalWidth => (width.space, None),
            &BoxLayout::FixedWidth(final_width) => {
                let natural_width = width.space;

                if final_width == natural_width {
                    (final_width, None)
                } else {
                    let stretch_needed = final_width - natural_width;

                    if final_width > natural_width {
                        (
                            final_width,
                            Some(set_glue(&stretch_needed, &width.stretch)),
                        )
                    } else {
                        (
                            final_width,
                            Some(set_glue(&stretch_needed, &width.shrink)),
                        )
                    }
                }
            }
            &BoxLayout::Spread(spread_needed) => {
                if spread_needed > Dimen::zero() {
                    (
                        width.space + spread_needed,
                        Some(set_glue(&spread_needed, &width.stretch)),
                    )
                } else {
                    (
                        width.space + spread_needed,
                        Some(set_glue(&spread_needed, &width.shrink)),
                    )
                }
            }
        };

        HorizontalBox {
            height: height,
            depth: depth,
            width: set_width,

            list: list,
            glue_set_ratio: set_ratio,
        }
    }

    fn parse_box_specification(&mut self) -> BoxLayout {
        if self.parse_optional_keyword_expanded("to") {
            let dimen = self.parse_dimen();
            self.parse_filler_expanded();
            BoxLayout::FixedWidth(dimen)
        } else if self.parse_optional_keyword_expanded("spread") {
            let dimen = self.parse_dimen();
            self.parse_filler_expanded();
            BoxLayout::Spread(dimen)
        } else {
            self.parse_filler_expanded();
            BoxLayout::NaturalWidth
        }
    }

    pub fn is_box_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&["hbox"])
    }

    pub fn parse_box(&mut self) -> TeXBox {
        let head = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&head, "hbox") {
            let layout = self.parse_box_specification();

            // We expect a { after the box specification
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::BeginGroup)) => (),
                _ => panic!("Expected { when parsing box"),
            }

            let hbox = self.parse_horizontal_box(&layout);

            // And there should always be a } after the horizontal list
            match self.lex_expanded_token() {
                Some(Token::Char(_, Category::EndGroup)) => (),
                _ => panic!("Expected } when parsing box"),
            }

            TeXBox::HorizontalBox(hbox)
        } else {
            panic!("unimplemented");
        }
    }

    // Used for early testing, when we're not going to be inspecting a whole
    // output box.
    pub fn parse_horizontal_box_to_chars(&mut self) -> Vec<char> {
        let hbox = self.parse_horizontal_box(&BoxLayout::NaturalWidth);
        hbox.to_chars()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dimension::{Dimen, Unit};
    use crate::testing::with_parser;

    #[test]
    fn it_parses_boxes_with_characters() {
        with_parser(&["gb%"], |parser| {
            let hbox = parser.parse_horizontal_box(&BoxLayout::NaturalWidth);

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

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
            let hbox = parser.parse_horizontal_box(&BoxLayout::NaturalWidth);

            assert_eq!(hbox.height, Dimen::zero());
            assert_eq!(hbox.depth, Dimen::zero());
            assert_eq!(hbox.width, Dimen::from_unit(3.0, Unit::Point));
        });
    }

    #[test]
    fn it_parses_boxes_with_glue_and_characters() {
        with_parser(&["b\\hskip 2pt g%"], |parser| {
            let hbox = parser.parse_horizontal_box(&BoxLayout::NaturalWidth);

            assert_eq!(hbox.list.len(), 3);

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

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
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();

            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

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
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 5.0))
            );
        });

        // Fill
        with_parser(&["a\\hskip 0pt plus1fill b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

            assert_eq!(hbox.width, fixed_width);
            assert_eq!(
                hbox.glue_set_ratio,
                Some(GlueSetRatio::from(GlueSetRatioKind::Fill, 5.0))
            );
        });

        // Filll
        with_parser(&["a\\hskip 0pt plus1filll b%"], |parser| {
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let fixed_width = metrics.get_width('a')
                + metrics.get_width('b')
                + Dimen::from_unit(5.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

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
                    parser.state.get_metrics_for_font("cmr10").unwrap();
                let fixed_width = metrics.get_width('a')
                    + metrics.get_width('b')
                    + Dimen::from_unit(6.0, Unit::Point);

                let hbox = parser
                    .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

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
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let fixed_width = metrics.get_width('a') + metrics.get_width('b')
                - Dimen::from_unit(1.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

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
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let fixed_width = metrics.get_width('a') + metrics.get_width('b')
                - Dimen::from_unit(4.0, Unit::Point);

            let hbox = parser
                .parse_horizontal_box(&BoxLayout::FixedWidth(fixed_width));

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
            let hbox = parser.parse_horizontal_box(&BoxLayout::Spread(
                Dimen::from_unit(6.0, Unit::Point),
            ));

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
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
            let hbox = parser.parse_horizontal_box(&BoxLayout::Spread(
                Dimen::from_unit(6.0, Unit::Point),
            ));

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
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
            let hbox = parser.parse_horizontal_box(&BoxLayout::Spread(
                Dimen::from_unit(-1.0, Unit::Point),
            ));

            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
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
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('b')
                + metrics.get_width('c');

            assert!(parser.is_box_head());
            let hbox = parser.parse_box();
            let TeXBox::HorizontalBox(hbox) = hbox;

            assert_eq!(hbox.list.len(), 3);
            assert_eq!(hbox.glue_set_ratio, None);
            assert_eq!(hbox.width, expected_width);
        });
    }

    #[test]
    fn it_parses_horizontal_boxes_with_fixed_width() {
        with_parser(&["\\hbox to20pt{a\\hskip 0pt plus1filc}%"], |parser| {
            assert!(parser.is_box_head());
            let hbox = parser.parse_box();
            let TeXBox::HorizontalBox(hbox) = hbox;

            assert_eq!(hbox.list.len(), 3);
            assert_eq!(hbox.width, Dimen::from_unit(20.0, Unit::Point));
        });
    }

    #[test]
    fn it_parses_horizontal_boxes_with_spread_width() {
        with_parser(&["\\hbox spread5pt{a\\hskip 0pt plus1filc}%"], |parser| {
            let metrics = parser.state.get_metrics_for_font("cmr10").unwrap();
            let expected_width = metrics.get_width('a')
                + metrics.get_width('c')
                + Dimen::from_unit(5.0, Unit::Point);

            assert!(parser.is_box_head());
            let hbox = parser.parse_box();
            let TeXBox::HorizontalBox(hbox) = hbox;

            assert_eq!(hbox.list.len(), 3);
            assert_eq!(hbox.width, expected_width);
        });
    }

    #[test]
    fn it_parses_chars_from_horizontal_boxes() {
        with_parser(&["a {b }c%"], |parser| {
            assert_eq!(
                parser.parse_horizontal_box_to_chars(),
                vec!['a', ' ', 'b', ' ', 'c']
            );
        });
    }
}

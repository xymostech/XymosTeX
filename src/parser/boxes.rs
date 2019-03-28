use crate::boxes::HorizontalBox;
use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    fn parse_horizontal_box(&mut self) -> HorizontalBox {
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

        HorizontalBox {
            height: height,
            depth: depth,
            width: width.space,

            list: list,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dimension::{Dimen, Unit};
    use crate::testing::with_parser;

    #[test]
    fn it_parses_boxes_with_characters() {
        with_parser(&["gb%"], |parser| {
            let hbox = parser.parse_horizontal_box();

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
            let hbox = parser.parse_horizontal_box();

            assert_eq!(hbox.height, Dimen::zero());
            assert_eq!(hbox.depth, Dimen::zero());
            assert_eq!(hbox.width, Dimen::from_unit(3.0, Unit::Point));
        });
    }

    #[test]
    fn it_parses_boxes_with_glue_and_characters() {
        with_parser(&["b\\hskip 2pt g%"], |parser| {
            let hbox = parser.parse_horizontal_box();

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
}

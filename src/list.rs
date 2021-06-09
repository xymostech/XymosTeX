use crate::boxes::TeXBox;
use crate::dimension::Dimen;
use crate::font::Font;
use crate::glue::Glue;
use crate::state::TeXState;

#[derive(Debug, PartialEq, Clone)]
pub enum HorizontalListElem {
    Char { chr: char, font: Font },
    HSkip(Glue),
    Box { tex_box: TeXBox, shift: Dimen },
}

impl HorizontalListElem {
    pub fn get_size(&self, state: &TeXState) -> (Dimen, Dimen, Glue) {
        match self {
            HorizontalListElem::Char { chr, font } => {
                let metrics = state.get_metrics_for_font(&font).unwrap();

                let height = metrics.get_height(*chr);
                let depth = metrics.get_depth(*chr);
                let width = metrics.get_width(*chr);

                (height, depth, Glue::from_dimen(width))
            }

            HorizontalListElem::HSkip(glue) => {
                (Dimen::zero(), Dimen::zero(), glue.clone())
            }

            HorizontalListElem::Box { tex_box, shift } => (
                if *tex_box.height() + *shift < Dimen::zero() {
                    Dimen::zero()
                } else {
                    *tex_box.height() + *shift
                },
                if *tex_box.depth() - *shift < Dimen::zero() {
                    Dimen::zero()
                } else {
                    *tex_box.depth() - *shift
                },
                Glue::from_dimen(*tex_box.width()),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerticalListElem {
    Box { tex_box: TeXBox, shift: Dimen },
    VSkip(Glue),
}

impl VerticalListElem {
    pub fn get_size(&self) -> (Glue, Dimen, Dimen) {
        match self {
            VerticalListElem::Box { tex_box, shift } => (
                Glue::from_dimen(*tex_box.height()),
                *tex_box.depth(),
                if *tex_box.width() + *shift < Dimen::zero() {
                    Dimen::zero()
                } else {
                    *tex_box.width() + *shift
                },
            ),

            VerticalListElem::VSkip(glue) => {
                (glue.clone(), Dimen::zero(), Dimen::zero())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::HorizontalBox;
    use crate::dimension::Unit;
    use crate::state::TeXState;

    #[test]
    fn it_calculates_horizontal_list_elem_sizes_with_shifts() {
        let mut hbox = HorizontalBox::empty();
        hbox.height = Dimen::from_unit(2.0, Unit::Point);
        hbox.depth = Dimen::from_unit(3.0, Unit::Point);
        hbox.width = Dimen::from_unit(4.0, Unit::Point);

        let state = TeXState::new();

        let tex_box = TeXBox::HorizontalBox(hbox);

        assert_eq!(
            HorizontalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::zero()
            }
            .get_size(&state),
            (
                Dimen::from_unit(2.0, Unit::Point),
                Dimen::from_unit(3.0, Unit::Point),
                Glue::from_dimen(Dimen::from_unit(4.0, Unit::Point)),
            )
        );

        assert_eq!(
            HorizontalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(1.0, Unit::Point)
            }
            .get_size(&state),
            (
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(2.0, Unit::Point),
                Glue::from_dimen(Dimen::from_unit(4.0, Unit::Point)),
            )
        );

        assert_eq!(
            HorizontalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(4.0, Unit::Point)
            }
            .get_size(&state),
            (
                Dimen::from_unit(6.0, Unit::Point),
                Dimen::from_unit(0.0, Unit::Point),
                Glue::from_dimen(Dimen::from_unit(4.0, Unit::Point)),
            )
        );

        assert_eq!(
            HorizontalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(-1.0, Unit::Point)
            }
            .get_size(&state),
            (
                Dimen::from_unit(1.0, Unit::Point),
                Dimen::from_unit(4.0, Unit::Point),
                Glue::from_dimen(Dimen::from_unit(4.0, Unit::Point)),
            )
        );

        assert_eq!(
            HorizontalListElem::Box {
                tex_box,
                shift: Dimen::from_unit(-3.0, Unit::Point)
            }
            .get_size(&state),
            (
                Dimen::from_unit(0.0, Unit::Point),
                Dimen::from_unit(6.0, Unit::Point),
                Glue::from_dimen(Dimen::from_unit(4.0, Unit::Point)),
            )
        );
    }

    #[test]
    fn it_calculates_vertical_list_elem_sizes_with_shifts() {
        let mut hbox = HorizontalBox::empty();
        hbox.height = Dimen::from_unit(2.0, Unit::Point);
        hbox.depth = Dimen::from_unit(3.0, Unit::Point);
        hbox.width = Dimen::from_unit(4.0, Unit::Point);

        let tex_box = TeXBox::HorizontalBox(hbox);

        assert_eq!(
            VerticalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::zero()
            }
            .get_size(),
            (
                Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point)),
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(4.0, Unit::Point),
            )
        );

        assert_eq!(
            VerticalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(1.0, Unit::Point)
            }
            .get_size(),
            (
                Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point)),
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(5.0, Unit::Point),
            )
        );

        assert_eq!(
            VerticalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(4.0, Unit::Point)
            }
            .get_size(),
            (
                Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point)),
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(8.0, Unit::Point),
            )
        );

        assert_eq!(
            VerticalListElem::Box {
                tex_box: tex_box.clone(),
                shift: Dimen::from_unit(-1.0, Unit::Point)
            }
            .get_size(),
            (
                Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point)),
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(3.0, Unit::Point),
            )
        );

        assert_eq!(
            VerticalListElem::Box {
                tex_box,
                shift: Dimen::from_unit(-5.0, Unit::Point)
            }
            .get_size(),
            (
                Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point)),
                Dimen::from_unit(3.0, Unit::Point),
                Dimen::from_unit(0.0, Unit::Point),
            )
        );
    }
}

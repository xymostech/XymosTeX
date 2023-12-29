use std::fmt;

use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen};
use crate::glue::Glue;
use crate::list::{HorizontalListElem, VerticalListElem};
use crate::state::TeXState;

#[derive(Debug, PartialEq, Clone)]
pub enum GlueSetRatioKind {
    Finite,
    Fil,
    Fill,
    Filll,
}

impl GlueSetRatioKind {
    // A utility method to map from the different kinds of FilKinds to the
    // different GlueSetRatioKinds.
    pub fn from_fil_kind(fil_kind: &FilKind) -> Self {
        match fil_kind {
            FilKind::Fil => GlueSetRatioKind::Fil,
            FilKind::Fill => GlueSetRatioKind::Fill,
            FilKind::Filll => GlueSetRatioKind::Filll,
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct GlueSetRatio {
    // What kinds of glue should be stretching. For instance, if this is
    // GlueSetRatioKind::Fil then only glues with fil stretch/shrink components
    // will be affected.
    kind: GlueSetRatioKind,
    // How much to stretch/shrink. Each increment of this value represents
    // 1/65536 of a unit of stretch. Measured in units of pt/pt for finite
    // stretching and pt/<fil/fill/filll> for infinite stretching.
    stretch: i32,
}

impl fmt::Debug for GlueSetRatio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlueSetRatio")
            .field("kind", &self.kind)
            .field(
                "stretch",
                &format!("{:.3}", (self.stretch as f64) / 65536.0),
            )
            .finish()
    }
}

impl GlueSetRatio {
    pub fn from(kind: GlueSetRatioKind, ratio: f64) -> GlueSetRatio {
        GlueSetRatio {
            kind,
            stretch: (ratio * 65536.0).round() as i32,
        }
    }

    fn multiply_spring_dimen(&self, spring_dimen: &SpringDimen) -> Dimen {
        match (&self.kind, spring_dimen) {
            (&GlueSetRatioKind::Finite, SpringDimen::Dimen(dimen)) => {
                *dimen * (self.stretch, 65536)
            }
            (
                &GlueSetRatioKind::Fil,
                SpringDimen::FilDimen(FilDimen(FilKind::Fil, fils)),
            ) => Dimen::from_scaled_points(
                ((*fils as i64) * (self.stretch as i64) / 65536) as i32,
            ),
            (
                &GlueSetRatioKind::Fill,
                SpringDimen::FilDimen(FilDimen(FilKind::Fill, fills)),
            ) => Dimen::from_scaled_points(
                ((*fills as i64) * (self.stretch as i64) / 65536) as i32,
            ),
            (
                &GlueSetRatioKind::Filll,
                SpringDimen::FilDimen(FilDimen(FilKind::Filll, fillls)),
            ) => Dimen::from_scaled_points(
                ((*fillls as i64) * (self.stretch as i64) / 65536) as i32,
            ),
            _ => Dimen::zero(),
        }
    }

    pub fn apply_to_glue(&self, glue: &Glue) -> Dimen {
        if self.stretch < 0 {
            glue.space + self.multiply_spring_dimen(&glue.shrink)
        } else {
            glue.space + self.multiply_spring_dimen(&glue.stretch)
        }
    }

    pub fn get_badness(&self) -> u64 {
        match self.kind {
            GlueSetRatioKind::Finite => (100.0
                * ((self.stretch as f64) / 65536.0).powi(3))
            .abs()
            .round()
            .min(10000.0) as u64,
            _ => 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum GlueSetResult {
    InsufficientShrink,
    ZeroStretch,
    ZeroShrink,
    GlueSetRatio(GlueSetRatio),
}

impl GlueSetResult {
    pub fn get_badness(&self) -> u64 {
        match self {
            GlueSetResult::InsufficientShrink => 10000,
            GlueSetResult::ZeroStretch => 10000,
            GlueSetResult::ZeroShrink => 10000,
            GlueSetResult::GlueSetRatio(glue_set_ratio) => {
                glue_set_ratio.get_badness()
            }
        }
    }

    pub fn to_glue_set_ratio(self) -> GlueSetRatio {
        match self {
            GlueSetResult::InsufficientShrink => {
                GlueSetRatio::from(GlueSetRatioKind::Finite, -1.0)
            }
            GlueSetResult::ZeroStretch => {
                GlueSetRatio::from(GlueSetRatioKind::Finite, 0.0)
            }
            GlueSetResult::ZeroShrink => {
                GlueSetRatio::from(GlueSetRatioKind::Finite, 0.0)
            }
            GlueSetResult::GlueSetRatio(glue_set_ratio) => glue_set_ratio,
        }
    }
}

// Given the amount of stretch/shrink needed to set a given box and the amount
// of stretch/shrink available, figure out the glue set ratio.
fn set_glue_for_positive_stretch(
    stretch_needed: &Dimen,
    stretch_available: &SpringDimen,
) -> GlueSetResult {
    match stretch_available {
        // If we have a finite amount of stretch/shrink available, then we set
        // a finite glue ratio but have some limits on how much we can
        // stretch/shrink
        SpringDimen::Dimen(stretch_dimen) => {
            if stretch_needed == &Dimen::zero() {
                GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                    GlueSetRatioKind::Finite,
                    0.0,
                ))
            } else if stretch_dimen == &Dimen::zero() {
                if stretch_needed < &Dimen::zero() {
                    GlueSetResult::ZeroShrink
                } else {
                    GlueSetResult::ZeroStretch
                }
            } else if stretch_needed / stretch_dimen < -1.0 {
                GlueSetResult::InsufficientShrink
            } else {
                GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                    GlueSetRatioKind::Finite,
                    stretch_needed / stretch_dimen,
                ))
            }
        }

        // If there's an infinite amount of stretch/shrink available, then we
        // can stretch/shrink as much as is needed with no limits.
        SpringDimen::FilDimen(stretch_fil_dimen) => {
            GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                GlueSetRatioKind::from_fil_kind(&stretch_fil_dimen.0),
                if stretch_fil_dimen.is_zero() {
                    0.0
                } else {
                    stretch_needed / stretch_fil_dimen
                },
            ))
        }
    }
}

pub fn set_glue_for_spread(spread: &Dimen, glue: &Glue) -> GlueSetResult {
    // If we're spreading the box, we just need to figure out if
    // we're stretching or shrinking, since we already know the
    // amount of spread.
    let stretch_or_shrink = if spread > &Dimen::zero() {
        &glue.stretch
    } else {
        &glue.shrink
    };

    set_glue_for_positive_stretch(&spread, stretch_or_shrink)
}

pub fn set_glue_for_dimen(final_dimen: &Dimen, glue: &Glue) -> GlueSetResult {
    // If we need to stretch, calculate the amount we need to
    // stretch.
    let stretch_needed = *final_dimen - glue.space;

    set_glue_for_spread(&stretch_needed, glue)
}

pub enum BoxLayout {
    Natural,
    Fixed(Dimen),
    Spread(Dimen),
}

/// Based on the layout of a box and the stretchable dimension, return the
/// resulting true dimension and the needed glue set ratio.
pub fn get_set_dimen_and_ratio(
    glue: Glue,
    layout: &BoxLayout,
) -> (Dimen, Option<GlueSetRatio>) {
    match *layout {
        // If we just want the box at its natural dimension, we just return the
        // "space" component of our dimension.
        BoxLayout::Natural => (glue.space, None),

        BoxLayout::Fixed(final_dimen) => {
            let natural_dimen = glue.space;

            // If the natural dimension of the box exactly equals the desired
            // dimension, then we don't need a glue set. This is probably very
            // unlikely to happen except in unique cases, like when the
            // dimension is 0.
            if final_dimen == natural_dimen {
                (final_dimen, None)
            } else {
                (
                    // The resulting box dimension is exactly the fixed
                    // dimension that was desired.
                    final_dimen,
                    Some(
                        set_glue_for_dimen(&final_dimen, &glue)
                            .to_glue_set_ratio(),
                    ),
                )
            }
        }
        BoxLayout::Spread(spread_needed) => {
            (
                // The final dimension is the natural dimension + spread
                glue.space + spread_needed,
                Some(
                    set_glue_for_spread(&spread_needed, &glue)
                        .to_glue_set_ratio(),
                ),
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalBox {
    pub height: Dimen,
    pub depth: Dimen,
    pub width: Dimen,

    pub list: Vec<HorizontalListElem>,
    // For each glue, this says how much the glue should stretch/shrink by.
    pub glue_set_ratio: Option<GlueSetRatio>,
}

impl HorizontalBox {
    #[cfg(test)]
    pub fn to_chars(&self) -> Vec<char> {
        // Since `to_chars()` is really just for early debugging, this is a
        // special rule for adding a space when we encounter an 'indent' box,
        // which is an empty box with positive width.
        if self.list.is_empty() && self.width > Dimen::zero() {
            return vec![' '];
        }

        self.list
            .iter()
            .flat_map(|elem| match elem {
                HorizontalListElem::Char { chr: ch, font: _ } => vec![*ch],
                HorizontalListElem::HSkip(_) => vec![' '],
                HorizontalListElem::Box { tex_box, shift: _ } => {
                    tex_box.to_chars()
                }
            })
            .collect()
    }

    /// Returns an empty, zero-size horizontal box.
    pub fn empty() -> Self {
        HorizontalBox {
            height: Dimen::zero(),
            depth: Dimen::zero(),
            width: Dimen::zero(),
            list: Vec::new(),
            glue_set_ratio: None,
        }
    }

    pub fn create_from_horizontal_list_with_layout(
        list: Vec<HorizontalListElem>,
        layout: &BoxLayout,
        state: &TeXState,
    ) -> HorizontalBox {
        // Keep track of the max height/depth and the total amount of width of;
        // the elements in the list.
        let mut height = Dimen::zero();
        let mut depth = Dimen::zero();
        let mut width = Glue::zero();

        for elem in &list {
            let (elem_height, elem_depth, elem_width) = elem.get_size(state);

            // Height and depth are just the maximum of all of the elements.
            if elem_height > height {
                height = elem_height;
            }
            if elem_depth > depth {
                depth = elem_depth;
            }

            // elem.get_size() returns a Glue for the width, so we just add up
            // all of the glue widths that are in the list.
            width = width + elem_width;
        }

        // Figure out the final width and glue set needed.
        let (set_width, set_ratio) = get_set_dimen_and_ratio(width, layout);

        HorizontalBox {
            height,
            depth,
            width: set_width,

            list,
            glue_set_ratio: set_ratio,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerticalBox {
    pub height: Dimen,
    pub depth: Dimen,
    pub width: Dimen,

    pub list: Vec<VerticalListElem>,
    // For each glue, this says how much the glue should stretch/shrink by.
    pub glue_set_ratio: Option<GlueSetRatio>,
}

impl VerticalBox {
    #[cfg(test)]
    pub fn to_chars(&self) -> Vec<char> {
        self.list
            .iter()
            // TODO(xymostech): Figure out a better way to insert a '\n' in
            // between each element here.
            .flat_map(|elem| match elem {
                VerticalListElem::VSkip(_) => vec![],
                VerticalListElem::Box { tex_box, shift: _ } => {
                    let mut vec = tex_box.to_chars();
                    vec.push('\n');
                    vec
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TeXBox {
    HorizontalBox(HorizontalBox),
    VerticalBox(VerticalBox),
}

impl TeXBox {
    pub fn height(&self) -> &Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &hbox.height,
            TeXBox::VerticalBox(vbox) => &vbox.height,
        }
    }

    pub fn width(&self) -> &Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &hbox.width,
            TeXBox::VerticalBox(vbox) => &vbox.width,
        }
    }

    pub fn depth(&self) -> &Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &hbox.depth,
            TeXBox::VerticalBox(vbox) => &vbox.depth,
        }
    }

    pub fn mut_height(&mut self) -> &mut Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &mut hbox.height,
            TeXBox::VerticalBox(vbox) => &mut vbox.height,
        }
    }

    pub fn mut_width(&mut self) -> &mut Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &mut hbox.width,
            TeXBox::VerticalBox(vbox) => &mut vbox.width,
        }
    }

    pub fn mut_depth(&mut self) -> &mut Dimen {
        match self {
            TeXBox::HorizontalBox(hbox) => &mut hbox.depth,
            TeXBox::VerticalBox(vbox) => &mut vbox.depth,
        }
    }

    // For early testing, we're not actually going to outputting a DVI file
    // with the correctly formatted text. So to test things, we'll just pull
    // out the contents of the box as a list of characters.
    #[cfg(test)]
    pub fn to_chars(&self) -> Vec<char> {
        match self {
            TeXBox::HorizontalBox(hbox) => hbox.to_chars(),
            TeXBox::VerticalBox(vbox) => vbox.to_chars(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use once_cell::sync::Lazy;

    use crate::dimension::Unit;
    use crate::font::Font;
    use crate::glue::Glue;

    static CMR10: Lazy<Font> = Lazy::new(|| Font {
        font_name: "cmr10".to_string(),
        scale: Dimen::from_unit(10.0, Unit::Point),
    });

    #[test]
    fn it_parses_to_chars() {
        let inner_hbox = TeXBox::HorizontalBox(HorizontalBox {
            width: Dimen::zero(),
            height: Dimen::zero(),
            depth: Dimen::zero(),

            list: vec![
                HorizontalListElem::Char {
                    chr: 'a',
                    font: CMR10.clone(),
                },
                HorizontalListElem::HSkip(Glue::from_dimen(Dimen::zero())),
                HorizontalListElem::Box {
                    tex_box: TeXBox::HorizontalBox(HorizontalBox {
                        width: Dimen::zero(),
                        height: Dimen::zero(),
                        depth: Dimen::zero(),

                        list: vec![
                            HorizontalListElem::Char {
                                chr: 'b',
                                font: CMR10.clone(),
                            },
                            HorizontalListElem::HSkip(Glue::from_dimen(
                                Dimen::zero(),
                            )),
                        ],
                        glue_set_ratio: None,
                    }),
                    shift: Dimen::zero(),
                },
                HorizontalListElem::Char {
                    chr: 'c',
                    font: CMR10.clone(),
                },
            ],
            glue_set_ratio: None,
        });

        let test_box = TeXBox::VerticalBox(VerticalBox {
            width: Dimen::zero(),
            height: Dimen::zero(),
            depth: Dimen::zero(),

            list: vec![
                VerticalListElem::Box {
                    tex_box: inner_hbox.clone(),
                    shift: Dimen::zero(),
                },
                VerticalListElem::VSkip(Glue::from_dimen(Dimen::zero())),
                VerticalListElem::Box {
                    tex_box: inner_hbox,
                    shift: Dimen::zero(),
                },
            ],
            glue_set_ratio: None,
        });

        #[rustfmt::skip]
        assert_eq!(
            test_box.to_chars(),
            vec![
                'a', ' ', 'b', ' ', 'c', '\n',
                'a', ' ', 'b', ' ', 'c', '\n',
            ]
        );
    }

    #[test]
    fn it_limits_glue_shrink_to_negative_one() {
        let glue = Glue {
            space: Dimen::from_unit(10.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::zero()),
            shrink: SpringDimen::Dimen(Dimen::from_unit(5.0, Unit::Point)),
        };

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(10.0, Unit::Point), &glue),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                GlueSetRatioKind::Finite,
                0.0
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(6.0, Unit::Point), &glue),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                GlueSetRatioKind::Finite,
                -4.0 / 5.0
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(5.0, Unit::Point), &glue),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                GlueSetRatioKind::Finite,
                -5.0 / 5.0
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(4.0, Unit::Point), &glue),
            GlueSetResult::InsufficientShrink,
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(4.0, Unit::Point), &glue)
                .to_glue_set_ratio(),
            GlueSetRatio::from(GlueSetRatioKind::Finite, -1.0),
        );

        let infinite_glue = Glue {
            space: Dimen::from_unit(10.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::zero()),
            shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.0)),
        };

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(4.0, Unit::Point),
                &infinite_glue
            )
            .to_glue_set_ratio(),
            GlueSetRatio::from(GlueSetRatioKind::Fil, -6.0),
        );
    }

    #[test]
    fn it_handles_glue_with_zero_shrink_and_stretch() {
        let fixed_glue = Glue {
            space: Dimen::from_unit(10.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::zero()),
            shrink: SpringDimen::Dimen(Dimen::zero()),
        };

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(10.0, Unit::Point),
                &fixed_glue
            ),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from(
                GlueSetRatioKind::Finite,
                0.0
            ))
        );

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(9.0, Unit::Point),
                &fixed_glue
            ),
            GlueSetResult::ZeroShrink,
        );

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(9.0, Unit::Point),
                &fixed_glue
            )
            .to_glue_set_ratio(),
            GlueSetRatio::from(GlueSetRatioKind::Finite, 0.0),
        );

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(11.0, Unit::Point),
                &fixed_glue
            ),
            GlueSetResult::ZeroStretch,
        );

        assert_eq!(
            set_glue_for_dimen(
                &Dimen::from_unit(11.0, Unit::Point),
                &fixed_glue
            )
            .to_glue_set_ratio(),
            GlueSetRatio::from(GlueSetRatioKind::Finite, 0.0),
        );
    }
}

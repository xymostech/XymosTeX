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

#[derive(Clone)]
pub struct GlueSetRatio {
    // What kinds of glue should be stretching. For instance, if this is
    // GlueSetRatioKind::Fil then only glues with fil stretch/shrink components
    // will be affected.
    kind: GlueSetRatioKind,
    // How much to stretch/shrink, as a ratio between the total amount of space
    // to take up per each unit of glue that is stretching/shrinking.
    stretch: (i32, i32),
}

impl fmt::Debug for GlueSetRatio {
    #[allow(clippy::disallowed_types)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlueSetRatio")
            .field("kind", &self.kind)
            .field(
                "stretch",
                &format!("{:?}", self.stretch.0 as f64 / self.stretch.1 as f64),
            )
            .finish()
    }
}

#[cfg(test)]
impl PartialEq for GlueSetRatio {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && (self.stretch.0 as i64 * other.stretch.1 as i64
                == other.stretch.0 as i64 * self.stretch.1 as i64)
    }
}

impl GlueSetRatio {
    pub fn from_scaled_ratio(
        kind: GlueSetRatioKind,
        ratio: (i32, i32),
    ) -> GlueSetRatio {
        if ratio.1 < 0 {
            GlueSetRatio::from_scaled_ratio(kind, (-ratio.0, -ratio.1))
        } else {
            GlueSetRatio {
                kind,
                stretch: ratio,
            }
        }
    }

    #[allow(clippy::disallowed_types)]
    fn multiply_spring_dimen(&self, spring_dimen: &SpringDimen) -> Dimen {
        match (&self.kind, spring_dimen) {
            (&GlueSetRatioKind::Finite, SpringDimen::Dimen(dimen)) => {
                // TeX uses 64-bit floating point numbers in this calculation,
                // so we need to also to ensure correct rounding.
                *dimen * (self.stretch.0 as f64 / self.stretch.1 as f64)
            }
            (
                &GlueSetRatioKind::Fil,
                SpringDimen::FilDimen(FilDimen(FilKind::Fil, fils)),
            ) => {
                // TODO(emily): figure out if this (and the two below this) also
                // should use floating point math
                Dimen::from_scaled_points(*fils)
                    * (self.stretch.0 as f64 / self.stretch.1 as f64)
            }
            (
                &GlueSetRatioKind::Fill,
                SpringDimen::FilDimen(FilDimen(FilKind::Fill, fills)),
            ) => {
                Dimen::from_scaled_points(*fills)
                    * (self.stretch.0 as f64 / self.stretch.1 as f64)
            }
            (
                &GlueSetRatioKind::Filll,
                SpringDimen::FilDimen(FilDimen(FilKind::Filll, fillls)),
            ) => {
                Dimen::from_scaled_points(*fillls)
                    * (self.stretch.0 as f64 / self.stretch.1 as f64)
            }
            _ => Dimen::zero(),
        }
    }

    pub fn apply_to_glue(&self, glue: &Glue) -> Dimen {
        if self.is_stretch() {
            glue.space + self.multiply_spring_dimen(&glue.stretch)
        } else {
            glue.space + self.multiply_spring_dimen(&glue.shrink)
        }
    }

    pub fn is_stretch(&self) -> bool {
        // self.stretch.0 should always be positive, but we check it just in
        // case
        (self.stretch.1 > 0) == (self.stretch.0 > 0)
    }

    // Returns the badness of a box given the glue set ratio of that box.
    pub fn get_badness(&self) -> u64 {
        // To quote the source code of TeX[1]:
        //
        // > The actual method used to compute the badness [...] produces an
        // > integer value that is a reasonably close approximation to
        // > $100(t/s)^3$, and all implementations of TeX should use precisely
        // > this method.
        //
        // "Reasonably close" is maybe stretching it a bit in the current age of
        // computers, but when badness starts getting large, accuracy is
        // probably not the most important feature (since your paragraphs
        // probably look terrible anyways!). This formula only uses integers, so
        // it is also very portable.
        //
        // In our case, self.stretch is already (t/s*65536), so we just use
        // t=self.stretch and s=65536.
        //
        // TODO(xymostech): We should probably represent these ratios by just
        // storing the numerator and denominator separately, to ensure that we
        // get this calculation exactly correct. For now, the approximation of
        // this slight approximation works fine.
        //
        // [1]: From TeX-Live's version at of TeX at line 2317:
        //      https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/tex.web#L2317
        match self.kind {
            GlueSetRatioKind::Finite => {
                let inf_bad = 10000;
                let r: u64;
                let t: u64 = self.stretch.0.unsigned_abs() as u64;
                let s: u64 = self.stretch.1.unsigned_abs() as u64;
                if t == 0 {
                    return 0;
                }

                // 297^3 = 99.94 * 2^{18}, so we can use it to scale up the
                // numerator and denominator before cubing to retain most of the
                // precision without using floats.
                if t <= 7230584 {
                    r = t * 297 / s;
                } else if s >= 1663497 {
                    r = t / (s / 297);
                } else {
                    r = t;
                }

                if r > 1290 {
                    return inf_bad;
                }

                // We add a little bit before dividing to compensate for 297^3
                // not being quite 100 * 2^{18}.
                (r * r * r + 0o400_000) / 0o1_000_000
            }
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub enum GlueSetResult {
    InsufficientShrink,
    ZeroStretch,
    ZeroShrink,
    GlueSetRatio(GlueSetRatio),
}

impl GlueSetResult {
    pub fn into_glue_set_ratio(self) -> GlueSetRatio {
        match self {
            GlueSetResult::InsufficientShrink => {
                GlueSetRatio::from_scaled_ratio(
                    GlueSetRatioKind::Finite,
                    (-1, 1),
                )
            }
            GlueSetResult::ZeroStretch => GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (0, 1),
            ),
            GlueSetResult::ZeroShrink => GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (0, 1),
            ),
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
                GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                    GlueSetRatioKind::Finite,
                    (0, 1),
                ))
            } else if stretch_dimen == &Dimen::zero() {
                if stretch_needed < &Dimen::zero() {
                    GlueSetResult::ZeroShrink
                } else {
                    GlueSetResult::ZeroStretch
                }
            } else {
                let ratio = stretch_needed / stretch_dimen;
                if ratio.0 < -ratio.1 {
                    GlueSetResult::InsufficientShrink
                } else {
                    GlueSetResult::GlueSetRatio(
                        GlueSetRatio::from_scaled_ratio(
                            GlueSetRatioKind::Finite,
                            ratio,
                        ),
                    )
                }
            }
        }

        // If there's an infinite amount of stretch/shrink available, then we
        // can stretch/shrink as much as is needed with no limits.
        SpringDimen::FilDimen(stretch_fil_dimen) => {
            GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::from_fil_kind(&stretch_fil_dimen.0),
                if stretch_fil_dimen.is_zero() {
                    (0, 1)
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

    set_glue_for_positive_stretch(spread, stretch_or_shrink)
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
                            .into_glue_set_ratio(),
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
                        .into_glue_set_ratio(),
                ),
            )
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
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

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
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

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
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
            GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (0, 1)
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(6.0, Unit::Point), &glue),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (-4, 5)
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(5.0, Unit::Point), &glue),
            GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (-5, 5)
            ))
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(4.0, Unit::Point), &glue),
            GlueSetResult::InsufficientShrink,
        );

        assert_eq!(
            set_glue_for_dimen(&Dimen::from_unit(4.0, Unit::Point), &glue)
                .into_glue_set_ratio(),
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (-1, 1)),
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
            .into_glue_set_ratio(),
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Fil, (-6, 1)),
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
            GlueSetResult::GlueSetRatio(GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (0, 1)
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
            .into_glue_set_ratio(),
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (0, 1)),
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
            .into_glue_set_ratio(),
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (0, 1)),
        );
    }

    #[test]
    fn it_correctly_calculates_badness_for_glue() {
        assert_eq!(
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (1, 1))
                .get_badness(),
            100
        );
        assert_eq!(
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (2, 1))
                .get_badness(),
            800
        );
        assert_eq!(
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (3, 1))
                .get_badness(),
            2698
        );
        assert_eq!(
            GlueSetRatio::from_scaled_ratio(GlueSetRatioKind::Finite, (4, 1))
                .get_badness(),
            6396
        );
    }

    #[test]
    fn it_correctly_rounds_when_scaling_dimensions() {
        // Weird artifacts of floating point rounding turn up when calculating
        // things very close to a 0.5 border. In all of these cases, we are
        // calculating
        //   2500001 / (2 * x) * x
        // for some x. This gives 1250000.5, which sometimes rounds and up and
        // sometimes rounds down based on the specific values.
        assert_eq!(
            GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (2500001, 1200780 * 2)
            )
            .multiply_spring_dimen(&SpringDimen::Dimen(
                Dimen::from_scaled_points(1200780)
            )),
            Dimen::from_scaled_points(1250000)
        );

        assert_eq!(
            GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (2500001, 1119682 * 2)
            )
            .multiply_spring_dimen(&SpringDimen::Dimen(
                Dimen::from_scaled_points(1119682)
            )),
            Dimen::from_scaled_points(1250000)
        );

        assert_eq!(
            GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (2500001, 19373 * 2)
            )
            .multiply_spring_dimen(&SpringDimen::Dimen(
                Dimen::from_scaled_points(19373)
            )),
            Dimen::from_scaled_points(1250001)
        );

        assert_eq!(
            GlueSetRatio::from_scaled_ratio(
                GlueSetRatioKind::Finite,
                (2500001, 455499 * 2)
            )
            .multiply_spring_dimen(&SpringDimen::Dimen(
                Dimen::from_scaled_points(455499)
            )),
            Dimen::from_scaled_points(1250001)
        );
    }
}

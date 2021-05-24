use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen};
use crate::glue::Glue;
use crate::list::{HorizontalListElem, VerticalListElem};

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

#[derive(Debug, PartialEq, Clone)]
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

impl GlueSetRatio {
    pub fn from(kind: GlueSetRatioKind, ratio: f64) -> GlueSetRatio {
        GlueSetRatio {
            kind,
            stretch: (ratio * 65536.0) as i32,
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
    pub fn to_chars(&self) -> Vec<char> {
        self.list
            .iter()
            // TODO(xymostech): Figure out a better way to insert a '\n' in
            // between each element here.
            .flat_map(|elem| match elem {
                VerticalListElem::VSkip(_) => vec![],
                VerticalListElem::Box(tex_box) => {
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

    use crate::dimension::Unit;
    use crate::font::Font;
    use crate::glue::Glue;

    lazy_static! {
        static ref CMR10: Font = Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        };
    }

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
                VerticalListElem::Box(inner_hbox.clone()),
                VerticalListElem::VSkip(Glue::from_dimen(Dimen::zero())),
                VerticalListElem::Box(inner_hbox),
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
}

use crate::dimension::{Dimen, FilKind};
use crate::list::HorizontalListElem;

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
            kind: kind,
            stretch: (ratio * 65536.0) as i32,
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
        self.list
            .iter()
            .flat_map(|elem| match elem {
                HorizontalListElem::Char { chr: ch, font: _ } => vec![*ch],
                HorizontalListElem::HSkip(_) => vec![' '],
                HorizontalListElem::Box(tex_box) => tex_box.to_chars(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TeXBox {
    HorizontalBox(HorizontalBox),
}

impl TeXBox {
    pub fn height(&self) -> &Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &hbox.height
    }

    pub fn width(&self) -> &Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &hbox.width
    }

    pub fn depth(&self) -> &Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &hbox.depth
    }

    pub fn mut_height(&mut self) -> &mut Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &mut hbox.height
    }

    pub fn mut_width(&mut self) -> &mut Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &mut hbox.width
    }

    pub fn mut_depth(&mut self) -> &mut Dimen {
        let TeXBox::HorizontalBox(hbox) = self;
        &mut hbox.depth
    }

    // For early testing, we're not actually going to outputting a DVI file
    // with the correctly formatted text. So to test things, we'll just pull
    // out the contents of the box as a list of characters.
    pub fn to_chars(&self) -> Vec<char> {
        let TeXBox::HorizontalBox(hbox) = self;
        hbox.to_chars()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::glue::Glue;

    #[test]
    fn it_parses_to_chars() {
        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            width: Dimen::zero(),
            height: Dimen::zero(),
            depth: Dimen::zero(),

            list: vec![
                HorizontalListElem::Char {
                    chr: 'a',
                    font: "cmr10".to_string(),
                },
                HorizontalListElem::HSkip(Glue::from_dimen(Dimen::zero())),
                HorizontalListElem::Box(TeXBox::HorizontalBox(HorizontalBox {
                    width: Dimen::zero(),
                    height: Dimen::zero(),
                    depth: Dimen::zero(),

                    list: vec![
                        HorizontalListElem::Char {
                            chr: 'b',
                            font: "cmr10".to_string(),
                        },
                        HorizontalListElem::HSkip(Glue::from_dimen(
                            Dimen::zero(),
                        )),
                    ],
                    glue_set_ratio: None,
                })),
                HorizontalListElem::Char {
                    chr: 'c',
                    font: "cmr10".to_string(),
                },
            ],
            glue_set_ratio: None,
        });

        assert_eq!(test_box.to_chars(), vec!['a', ' ', 'b', ' ', 'c']);
    }
}

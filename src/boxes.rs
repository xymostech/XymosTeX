use crate::dimension::{Dimen, FilKind};
use crate::list::HorizontalListElem;

#[derive(Debug, PartialEq)]
pub enum GlueSetRatioKind {
    Dimen,
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

#[derive(Debug, PartialEq)]
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

pub struct HorizontalBox {
    pub height: Dimen,
    pub depth: Dimen,
    pub width: Dimen,

    pub list: Vec<HorizontalListElem>,
    // For each glue, this says how much the glue should stretch/shrink by.
    pub glue_set_ratio: Option<GlueSetRatio>,
}

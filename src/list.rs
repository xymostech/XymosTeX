use crate::boxes::TeXBox;
use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::state::TeXState;

#[derive(Debug, PartialEq, Clone)]
pub enum HorizontalListElem {
    Char { chr: char, font: String },
    HSkip(Glue),
    Box(TeXBox),
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

            HorizontalListElem::Box(tex_box) => (
                *tex_box.height(),
                *tex_box.depth(),
                Glue::from_dimen(*tex_box.width()),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerticalListElem {
    Box(TeXBox),
    VSkip(Glue),
}

impl VerticalListElem {
    pub fn get_size(&self) -> (Glue, Dimen, Dimen) {
        match self {
            VerticalListElem::Box(tex_box) => (
                Glue::from_dimen(*tex_box.height()),
                *tex_box.depth(),
                *tex_box.width(),
            ),

            VerticalListElem::VSkip(glue) => {
                (glue.clone(), Dimen::zero(), Dimen::zero())
            }
        }
    }
}

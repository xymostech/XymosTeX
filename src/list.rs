use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::state::TeXState;

#[derive(Debug, PartialEq)]
pub enum HorizontalListElem {
    Char { chr: char, font: String },
    HSkip(Glue),
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
        }
    }
}

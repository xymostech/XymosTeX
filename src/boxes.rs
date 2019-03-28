use crate::dimension::Dimen;
use crate::list::HorizontalListElem;

pub struct HorizontalBox {
    pub height: Dimen,
    pub depth: Dimen,
    pub width: Dimen,

    pub list: Vec<HorizontalListElem>,
}

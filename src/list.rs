use crate::glue::Glue;

#[derive(Debug, PartialEq)]
pub enum HorizontalListElem {
    Char { chr: char, font: String },
    HSkip(Glue),
}

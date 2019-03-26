use crate::glue::Glue;

#[derive(Debug, PartialEq)]
pub enum HorizontalListElem {
    Char(char),
    HSkip(Glue),
}

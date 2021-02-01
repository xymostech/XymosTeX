use crate::dimension::Dimen;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Font {
    pub font_name: String,
    pub scale: Dimen,
}

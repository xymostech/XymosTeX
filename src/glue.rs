use crate::dimension::{Dimen, SpringDimen};

#[derive(Debug, PartialEq, Eq)]
pub struct Glue {
    pub space: Dimen,
    pub stretch: SpringDimen,
    pub shrink: SpringDimen,
}

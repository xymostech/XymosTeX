use std::ops::{Add, Sub};

use crate::dimension::{Dimen, SpringDimen};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Glue {
    pub space: Dimen,
    pub stretch: SpringDimen,
    pub shrink: SpringDimen,
}

impl Glue {
    pub fn zero() -> Glue {
        Self::from_dimen(Dimen::zero())
    }

    pub fn from_dimen(dimen: Dimen) -> Glue {
        Glue {
            space: dimen,
            stretch: SpringDimen::Dimen(Dimen::zero()),
            shrink: SpringDimen::Dimen(Dimen::zero()),
        }
    }
}

impl Add for Glue {
    type Output = Glue;

    fn add(mut self, other: Glue) -> Glue {
        self.space = self.space + other.space;
        self.stretch = self.stretch + other.stretch;
        self.shrink = self.shrink + other.shrink;
        self
    }
}

impl Sub for Glue {
    type Output = Glue;

    // We dispatch to the Add impl to do subtraction here, so this isn't
    // suspicious
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, mut other: Glue) -> Glue {
        other.space = other.space * -1;
        other.stretch = other.stretch * -1;
        other.shrink = other.shrink * -1;
        self + other
    }
}

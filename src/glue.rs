use std::ops::{Add, Mul, Sub};

use crate::dimension::{Dimen, MuDimen, SpringDimen};

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

impl Mul<i32> for Glue {
    type Output = Glue;

    fn mul(mut self, other: i32) -> Glue {
        self.space = self.space * other;
        self.stretch = self.stretch * other;
        self.shrink = self.shrink * other;
        self
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MuGlue {
    pub space: MuDimen,
    pub stretch: MuDimen,
    pub shrink: MuDimen,
}

impl MuGlue {
    pub fn to_glue(&self, quad: Dimen) -> Glue {
        Glue {
            space: self.space.to_dimen(quad),
            stretch: SpringDimen::Dimen(self.stretch.to_dimen(quad)),
            shrink: SpringDimen::Dimen(self.shrink.to_dimen(quad)),
        }
    }
}

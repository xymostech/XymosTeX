use std::cmp::{Ordering, PartialOrd};
use std::ops::{Add, Div, Mul, Sub};

static DIMEN_MAX: i32 = (1 << 30) - 1;
static DIMEN_MIN: i32 = 1 - (1 << 30);

#[derive(Debug, Clone, Copy)]
pub enum Unit {
    Point,
    Pica,
    Inch,
    BigPoint,
    Centimeter,
    Millimeter,
    DidotPoint,
    Cicero,
    ScaledPoint,
}

// Return a fractional scale to convert from the passed in unit into scaled
// points. E.g. a return value of (7, 3) would indicate that there are 7/3 of
// those units per scaled point.
fn get_scale(unit: Unit) -> (f64, f64) {
    match unit {
        Unit::Point => (65536.0, 1.0),
        Unit::Pica => (12.0 * 65536.0, 1.0),
        Unit::Inch => (65536.0 * 7227.0, 100.0),
        Unit::BigPoint => (65536.0 * 7227.0, 72.0 * 100.0),
        Unit::Centimeter => (65536.0 * 7227.0, 254.0),
        Unit::Millimeter => (65536.0 * 7227.0, 2540.0),
        Unit::DidotPoint => (65536.0 * 1238.0, 1157.0),
        Unit::Cicero => (65536.0 * 1238.0 * 12.0, 1157.0),
        Unit::ScaledPoint => (1.0, 1.0),
    }
}

// Represents a dimension in terms of a number of scaled points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimen(i32);

impl Dimen {
    fn validate(self) -> Dimen {
        assert!(
            DIMEN_MIN <= self.0 && self.0 <= DIMEN_MAX,
            "Dimension too large"
        );
        self
    }

    pub fn zero() -> Dimen {
        Dimen(0)
    }

    // Given a number of a given unit, create a Dimen.
    pub fn from_unit(num: f64, from_unit: Unit) -> Dimen {
        let scale = get_scale(from_unit);
        Dimen((num * scale.0 / scale.1) as i32).validate()
    }

    // Given a Dimen and a unit to convert that to, returns the amount of that unit
    // that are in that Dimen.
    fn to_unit(&self, to_unit: Unit) -> f64 {
        let scale = get_scale(to_unit);
        (self.0 as f64) * scale.1 / scale.0
    }
}

impl PartialOrd for Dimen {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Add for Dimen {
    type Output = Dimen;
    fn add(self, other: Dimen) -> Dimen {
        Dimen(self.0 + other.0).validate()
    }
}

impl Sub for Dimen {
    type Output = Dimen;
    fn sub(self, other: Dimen) -> Dimen {
        Dimen(self.0 - other.0).validate()
    }
}

impl Mul<i32> for Dimen {
    type Output = Dimen;

    fn mul(self, other: i32) -> Dimen {
        Dimen(self.0 * other).validate()
    }
}

impl Div<i32> for Dimen {
    type Output = Dimen;

    fn div(self, other: i32) -> Dimen {
        Dimen(self.0 / other).validate()
    }
}

impl Div<&Dimen> for &Dimen {
    type Output = f64;

    fn div(self, other: &Dimen) -> f64 {
        (self.0 as f64) / (other.0 as f64)
    }
}

impl Div<&FilDimen> for &Dimen {
    type Output = f64;

    fn div(self, other: &FilDimen) -> f64 {
        (self.0 as f64) / (other.1 as f64)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FilKind {
    Fil,
    Fill,
    Filll,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FilDimen(pub FilKind, i32);

impl FilDimen {
    pub fn new(kind: FilKind, value: f64) -> Self {
        FilDimen(kind, (value * 65536.0) as i32)
    }
}

impl Add for FilDimen {
    type Output = FilDimen;

    fn add(self, other: FilDimen) -> FilDimen {
        match self {
            // Fillls are larger than any amount of fill or fil
            FilDimen(FilKind::Filll, a) => match other {
                FilDimen(FilKind::Filll, b) => FilDimen(FilKind::Filll, a + b),
                _ => FilDimen(FilKind::Filll, a),
            },

            // Fills are larger than fils but smaller than any filll
            FilDimen(FilKind::Fill, a) => match other {
                FilDimen(FilKind::Filll, _) => other,
                FilDimen(FilKind::Fill, b) => FilDimen(FilKind::Fill, a + b),
                _ => FilDimen(FilKind::Fill, a),
            },

            // Fills are smaller than any filll or fill
            FilDimen(FilKind::Fil, a) => match other {
                FilDimen(FilKind::Filll, _) => other,
                FilDimen(FilKind::Fill, _) => other,
                FilDimen(FilKind::Fil, b) => FilDimen(FilKind::Fil, a + b),
            },
        }
    }
}

impl Mul<i32> for FilDimen {
    type Output = FilDimen;

    fn mul(self, other: i32) -> FilDimen {
        FilDimen(self.0, self.1 * other)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SpringDimen {
    Dimen(Dimen),
    FilDimen(FilDimen),
}

impl Add for SpringDimen {
    type Output = SpringDimen;

    fn add(self, other: SpringDimen) -> SpringDimen {
        match self {
            SpringDimen::Dimen(a) => match other {
                SpringDimen::Dimen(b) => SpringDimen::Dimen(a + b),
                // Since a FilDimen represents an infinite dimension, when
                // adding them we don't need to keep track of the non-infinite
                // part of the dimension. So, we just return the infinite part
                // here.
                SpringDimen::FilDimen(_) => other,
            },
            SpringDimen::FilDimen(a) => match other {
                // Same as above.
                SpringDimen::Dimen(_) => SpringDimen::FilDimen(a),
                SpringDimen::FilDimen(b) => SpringDimen::FilDimen(a + b),
            },
        }
    }
}

impl Mul<i32> for SpringDimen {
    type Output = SpringDimen;

    fn mul(self, other: i32) -> SpringDimen {
        match self {
            SpringDimen::FilDimen(fil) => SpringDimen::FilDimen(fil * other),
            SpringDimen::Dimen(dimen) => SpringDimen::Dimen(dimen * other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_converts_units_to_dimens() {
        assert_eq!(Dimen::from_unit(1.0, Unit::Point), Dimen(65536));
        assert_eq!(Dimen::from_unit(10.0, Unit::Point), Dimen(655360));

        assert_eq!(Dimen::from_unit(1.0, Unit::Pica), Dimen(786432));
        assert_eq!(Dimen::from_unit(10.0, Unit::Pica), Dimen(7864320));

        assert_eq!(Dimen::from_unit(1.0, Unit::Inch), Dimen(4736286));
        assert_eq!(Dimen::from_unit(10.0, Unit::Inch), Dimen(47362867));

        assert_eq!(Dimen::from_unit(1.0, Unit::BigPoint), Dimen(65781));
        assert_eq!(Dimen::from_unit(10.0, Unit::BigPoint), Dimen(657817));

        assert_eq!(Dimen::from_unit(1.0, Unit::Centimeter), Dimen(1864679));
        assert_eq!(Dimen::from_unit(10.0, Unit::Centimeter), Dimen(18646798));

        assert_eq!(Dimen::from_unit(1.0, Unit::Millimeter), Dimen(186467));
        assert_eq!(Dimen::from_unit(10.0, Unit::Millimeter), Dimen(1864679));

        assert_eq!(Dimen::from_unit(1.0, Unit::DidotPoint), Dimen(70124));
        assert_eq!(Dimen::from_unit(100.0, Unit::DidotPoint), Dimen(7012408));

        assert_eq!(Dimen::from_unit(1.0, Unit::Cicero), Dimen(841489));
        assert_eq!(Dimen::from_unit(100.0, Unit::Cicero), Dimen(84148903));

        assert_eq!(Dimen::from_unit(1.0, Unit::ScaledPoint), Dimen(1));
        assert_eq!(Dimen::from_unit(10.0, Unit::ScaledPoint), Dimen(10));
    }

    // The values that we get out of conversions aren't going to be exactly the
    // same due to floating point inaccuracies, so we at least want to be
    // within some error. This checks that two values are within 1sp per pt.
    fn assert_close(a: f64, b: f64) {
        assert!(b - 1.0 / 65536.0 <= a && a <= b + 1.0 / 65536.0);
    }

    #[test]
    fn it_converts_dimens_to_units() {
        for unit in &[
            Unit::Point,
            Unit::Pica,
            Unit::Inch,
            Unit::BigPoint,
            Unit::Centimeter,
            Unit::Millimeter,
            Unit::DidotPoint,
            Unit::Cicero,
            Unit::ScaledPoint,
        ] {
            assert_close(Dimen::from_unit(10.0, *unit).to_unit(*unit), 10.0);
        }

        assert_close(Dimen(1000000000).to_unit(Unit::Point), 15258.78906);
    }

    #[test]
    fn it_supports_arithmetic() {
        assert_eq!(Dimen(1234) + Dimen(2345), Dimen(3579));
        assert_eq!(Dimen(2345) - Dimen(1234), Dimen(1111));
        assert_eq!(Dimen(1234) * 3, Dimen(3702));
        assert_eq!(Dimen(12345) / 2, Dimen(6172));
    }

    #[test]
    #[should_panic(expected = "Dimension too large")]
    fn it_checks_large_dimensions() {
        Dimen(1073741824).validate();
    }

    #[test]
    fn it_supports_negative_dimens() {
        assert_eq!(Dimen::from_unit(-123.0, Unit::Point), Dimen(-8060928));
    }

    #[test]
    fn it_adds_spring_dimens() {
        // dimen + dimen = dimen
        assert_eq!(
            SpringDimen::Dimen(Dimen(1234)) + SpringDimen::Dimen(Dimen(2345)),
            SpringDimen::Dimen(Dimen(3579))
        );

        // dimen + fil = fil
        assert_eq!(
            SpringDimen::Dimen(Dimen(1234))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
        );

        // dimen + fil = fil
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
                + SpringDimen::Dimen(Dimen(1234)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
        );

        // fil + fil = fil
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
                + SpringDimen::Dimen(Dimen(1234)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
        );

        // fil + fill = fill
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3))
        );

        // fill + fil = fill
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3))
        );

        // filll + fill = filll
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
        );

        // fill + filll = filll
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fill, 2.3))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
        );

        // fil + filll = filll
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
        );

        // filll + fil = filll
        assert_eq!(
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
                + SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.2)),
            SpringDimen::FilDimen(FilDimen::new(FilKind::Filll, 3.4))
        );
    }
}

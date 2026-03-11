#[derive(Debug, PartialEq)]
pub struct Fixnum(u32);

const FIXNUM_SIGN_MASK: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
const FIXNUM_NUMERIC_MASK: u32 = 0b0111_1111_1111_1111_1111_1111_1111_1111;

impl Fixnum {
    pub fn from_raw(raw: u32) -> Self {
        Fixnum(raw)
    }

    #[cfg(test)]
    pub fn from_float(mut float: f64) -> Self {
        let sign = if float < 0.0 {
            float *= -1.0;
            FIXNUM_SIGN_MASK
        } else {
            0
        };

        let integer_repr = (float * ((1 << 20) as f64)) as u32;
        if integer_repr > FIXNUM_NUMERIC_MASK {
            Fixnum(sign | FIXNUM_NUMERIC_MASK)
        } else {
            Fixnum(sign | integer_repr)
        }
    }

    #[allow(clippy::disallowed_types)]
    pub fn as_float(&self) -> f64 {
        let (numerator, denominator) = self.as_ratio();
        (numerator as f64) / (denominator as f64)
    }

    pub fn as_ratio(&self) -> (i64, i64) {
        if self.0 & FIXNUM_SIGN_MASK == FIXNUM_SIGN_MASK {
            let num = ((!self.0 + 1) & FIXNUM_NUMERIC_MASK) as i64;
            (-num, 1 << 20)
        } else {
            (self.0 as i64, 1 << 20)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Fixnum;

    #[test]
    fn it_reads_fixnums() {
        assert_eq!(
            Fixnum::from_raw(0b0000_0000_0000_0000_0000_0000_0000_0000)
                .as_float(),
            0.0
        );
        assert_eq!(
            Fixnum::from_raw(0b0100_0000_0000_1000_0000_0000_0000_0000)
                .as_float(),
            1024.5
        );
        assert_eq!(
            Fixnum::from_raw(0b1000_0000_0000_0000_0000_0000_0000_0000)
                .as_float(),
            -0.0
        );
        assert_eq!(
            Fixnum::from_raw(0b1000_0000_0000_0000_0000_0000_0000_0001)
                .as_float(),
            -2048.0 + 1.0 / (1 << 20) as f64
        );
    }
}

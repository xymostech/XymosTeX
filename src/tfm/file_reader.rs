use std::io;

struct TeXFileReader<T: io::Read> {
    reader: T,
}

const FIXNUM_SIGN_MASK: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
const FIXNUM_INT_MASK: u32 = 0b0111_1111_1111_0000_0000_0000_0000_0000;
const FIXNUM_FRAC_MASK: u32 = 0b0000_0000_0000_1111_1111_1111_1111_1111;

impl<T: io::Read> TeXFileReader<T> {
    fn new(reader: T) -> TeXFileReader<T> {
        TeXFileReader { reader: reader }
    }

    fn read_32bit_int(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;

        Ok(((buf[0] as u32) << 24)
            + ((buf[1] as u32) << 16)
            + ((buf[2] as u32) << 8)
            + (buf[3] as u32))
    }

    fn read_16bit_int(&mut self) -> io::Result<u16> {
        let mut buf = [0; 2];
        self.reader.read_exact(&mut buf)?;

        Ok(((buf[0] as u16) << 8) + (buf[1] as u16))
    }

    fn read_8bit_int(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.reader.read_exact(&mut buf)?;

        Ok(buf[0])
    }

    fn read_fixnum(&mut self) -> io::Result<f64> {
        let mut num = self.read_32bit_int()?;

        let sign = if num & FIXNUM_SIGN_MASK == FIXNUM_SIGN_MASK {
            num = !num + 1;
            -1.0
        } else {
            1.0
        };

        let int_part: f64 = ((num & FIXNUM_INT_MASK) >> 20) as f64;
        let frac_part: f64 =
            ((num & FIXNUM_FRAC_MASK) as f64) / ((1 << 20) as f64);

        Ok(sign * (int_part + frac_part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRead {
        buf: Vec<u8>,
        index: usize,
    }

    impl FakeRead {
        fn new(v: Vec<u8>) -> FakeRead {
            FakeRead { buf: v, index: 0 }
        }
    }

    impl io::Read for FakeRead {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.len() == 0 {
                Ok(0)
            } else if self.index >= self.buf.len() {
                Ok(0)
            } else {
                buf[0] = self.buf[self.index];
                self.index += 1;
                Ok(1)
            }
        }
    }

    #[test]
    fn it_reads_integers_and_fixnums() {
        let mut reader = TeXFileReader::new(FakeRead::new(vec![
            // 8 bit ints
            0x00,
            0xff,
            // 16 bit ints
            0x00,
            0x00,
            0x00,
            0xff,
            0xff,
            0xff,
            // 32 bit ints
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            // fixnums
            0b0000_0000,
            0b0000_0000,
            0b0000_0000,
            0b0000_0000,
            0b0100_0000,
            0b0000_1000,
            0b0000_0000,
            0b0000_0000,
            0b1000_0000,
            0b0000_0000,
            0b0000_0000,
            0b0000_0000,
            0b1000_0000,
            0b0000_0000,
            0b0000_0000,
            0b0000_0001,
        ]));

        assert_eq!(reader.read_8bit_int().unwrap(), 0);
        assert_eq!(reader.read_8bit_int().unwrap(), 255);

        assert_eq!(reader.read_16bit_int().unwrap(), 0);
        assert_eq!(reader.read_16bit_int().unwrap(), 255);
        assert_eq!(reader.read_16bit_int().unwrap(), 65535);

        assert_eq!(reader.read_32bit_int().unwrap(), 0);
        assert_eq!(reader.read_32bit_int().unwrap(), 255);
        assert_eq!(reader.read_32bit_int().unwrap(), 4294967295);

        assert_eq!(reader.read_fixnum().unwrap(), 0.0);
        assert_eq!(reader.read_fixnum().unwrap(), 1024.5);
        assert_eq!(reader.read_fixnum().unwrap(), -0.0);
        assert_eq!(
            reader.read_fixnum().unwrap(),
            -2048.0 + 1.0 / (1 << 20) as f64
        );
    }
}

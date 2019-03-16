use std::io;

pub struct TeXFileReader<T: io::Read> {
    reader: T,
}

const FIXNUM_SIGN_MASK: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
const FIXNUM_INT_MASK: u32 = 0b0111_1111_1111_0000_0000_0000_0000_0000;
const FIXNUM_FRAC_MASK: u32 = 0b0000_0000_0000_1111_1111_1111_1111_1111;

impl<T: io::Read> TeXFileReader<T> {
    pub fn new(reader: T) -> TeXFileReader<T> {
        TeXFileReader { reader: reader }
    }

    pub fn read_32bit_int(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;

        Ok(((buf[0] as u32) << 24)
            + ((buf[1] as u32) << 16)
            + ((buf[2] as u32) << 8)
            + (buf[3] as u32))
    }

    pub fn read_16bit_int(&mut self) -> io::Result<u16> {
        let mut buf = [0; 2];
        self.reader.read_exact(&mut buf)?;

        Ok(((buf[0] as u16) << 8) + (buf[1] as u16))
    }

    pub fn read_8bit_int(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.reader.read_exact(&mut buf)?;

        Ok(buf[0])
    }

    pub fn read_fixnum(&mut self) -> io::Result<f64> {
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

    // Reads a string from the file with a maximum length of max_len. Exactly
    // max_len bytes are read from the file, but the length of the resulting
    // string is found in the first byte that is read, so the final length of
    // the string is anywhere between 0 and max_len - 1 bytes.
    pub fn read_string(&mut self, max_len: usize) -> io::Result<String> {
        let str_len = self.read_8bit_int()? as usize;

        assert!(
            str_len < max_len,
            "Invalid string length: {} vs {}",
            str_len,
            max_len
        );

        let mut buf: Vec<u8> = Vec::new();
        buf.resize(max_len - 1, 0);
        self.reader.read_exact(&mut buf[..])?;
        buf.resize(str_len, 0);

        match String::from_utf8(buf) {
            Ok(string) => Ok(string),
            Err(_) => {
                Err(io::Error::new(io::ErrorKind::Other, "Invalid utf-8"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_integers_and_fixnums() {
        #[rustfmt::skip]
        let mut reader = TeXFileReader::new(&[
            // 8 bit ints
            0x00,
            0xff,
            // 16 bit ints
            0x00, 0x00, 0x00,
            0xff, 0xff, 0xff,
            // 32 bit ints
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xff,
            0xff, 0xff, 0xff, 0xff,
            // fixnums
            0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
            0b0100_0000, 0b0000_1000, 0b0000_0000, 0b0000_0000,
            0b1000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000,
            0b1000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0001,
        ][..]);

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

    #[test]
    fn it_reads_strings() {
        #[rustfmt::skip]
        let mut reader = TeXFileReader::new(&[
            // 8 byte empty string
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // 8 byte string "boo"
            0x03, 0x62, 0x6f, 0x6f, 0x00, 0x00, 0x00, 0x00,
            // 8 byte string "hello!!"
            0x07, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x21, 0x21,
        ][..]);

        assert_eq!(&reader.read_string(8).unwrap(), "");
        assert_eq!(&reader.read_string(8).unwrap(), "boo");
        assert_eq!(&reader.read_string(8).unwrap(), "hello!!");
    }
}

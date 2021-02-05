use std::io;

/// A wrapper around a reader that provides methods for easily reading the
/// specific bits of data the way they are stored in a DVI file.
pub struct DVIFileReader<T: io::Read> {
    reader: T,
}

// Generate a generic read_<n>_bytes_<signed/unsigned> function using
// <type>::from_be_bytes.
macro_rules! generate_int_reader_func {
    (fn $func_name:ident() -> $return_type:ident, $size:expr) => {
        pub fn $func_name(&mut self) -> io::Result<$return_type> {
            let mut buf = [0; $size];
            self.reader.read_exact(&mut buf)?;
            Ok($return_type::from_be_bytes(buf))
        }
    };
}

impl<T: io::Read> DVIFileReader<T> {
    pub fn new(reader: T) -> Self {
        DVIFileReader { reader }
    }

    generate_int_reader_func!(fn read_1_byte_unsigned() -> u8, 1);
    generate_int_reader_func!(fn read_2_bytes_unsigned() -> u16, 2);
    generate_int_reader_func!(fn read_4_bytes_unsigned() -> u32, 4);

    // This cannot be generated using generate_int_reader_func!() because there
    // is no u24 type.
    pub fn read_3_bytes_unsigned(&mut self) -> io::Result<u32> {
        let mut buf = [0; 3];
        self.reader.read_exact(&mut buf)?;
        let final_buf = [0, buf[0], buf[1], buf[2]];
        Ok(u32::from_be_bytes(final_buf))
    }

    generate_int_reader_func!(fn read_1_byte_signed() -> i8, 1);
    generate_int_reader_func!(fn read_2_bytes_signed() -> i16, 2);
    generate_int_reader_func!(fn read_4_bytes_signed() -> i32, 4);

    // This cannot be generated using generate_int_reader_func!() because there
    // is no i24 type.
    pub fn read_3_bytes_signed(&mut self) -> io::Result<i32> {
        let mut buf = [0; 3];
        self.reader.read_exact(&mut buf)?;
        let final_buf = [buf[0], buf[1], buf[2], 0];
        Ok(i32::from_be_bytes(final_buf) >> 8)
    }

    pub fn read_array(&mut self, size: usize) -> io::Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(size, 0);
        self.reader.read_exact(&mut buf[..])?;
        Ok(buf)
    }

    pub fn read_string(&mut self, size: usize) -> io::Result<String> {
        let arr = self.read_array(size)?;
        match String::from_utf8(arr) {
            Ok(string) => Ok(string),
            Err(err) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Error parsing utf-8: {}", err),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_unsigned_numbers() {
        #[rustfmt::skip]
        let mut reader = DVIFileReader::new(&[
            0x01,
            0xff,

            0x00, 0x01,
            0x01, 0x00,
            0xff, 0xff,

            0x00, 0x00, 0x01,
            0x00, 0x01, 0x00,
            0x01, 0x00, 0x00,
            0xff, 0xff, 0xff,

            0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00,
            0xff, 0xff, 0xff, 0xff,
        ][..]);

        assert_eq!(reader.read_1_byte_unsigned().unwrap(), 1);
        assert_eq!(reader.read_1_byte_unsigned().unwrap(), 255);

        assert_eq!(reader.read_2_bytes_unsigned().unwrap(), 1);
        assert_eq!(reader.read_2_bytes_unsigned().unwrap(), 256);
        assert_eq!(reader.read_2_bytes_unsigned().unwrap(), 65535);

        assert_eq!(reader.read_3_bytes_unsigned().unwrap(), 1);
        assert_eq!(reader.read_3_bytes_unsigned().unwrap(), 256);
        assert_eq!(reader.read_3_bytes_unsigned().unwrap(), 65536);
        assert_eq!(reader.read_3_bytes_unsigned().unwrap(), 16777215);

        assert_eq!(reader.read_4_bytes_unsigned().unwrap(), 1);
        assert_eq!(reader.read_4_bytes_unsigned().unwrap(), 256);
        assert_eq!(reader.read_4_bytes_unsigned().unwrap(), 65536);
        assert_eq!(reader.read_4_bytes_unsigned().unwrap(), 16777216);
        assert_eq!(reader.read_4_bytes_unsigned().unwrap(), 4294967295);
    }

    #[test]
    fn it_reads_signed_numbers() {
        #[rustfmt::skip]
        let mut reader = DVIFileReader::new(&[
            0x01,
            0x7f,
            0x80,
            0xff,

            0x00, 0x01,
            0x7f, 0xff,
            0x80, 0x00,
            0xff, 0xff,

            0x00, 0x00, 0x01,
            0x7f, 0xff, 0xff,
            0x80, 0x00, 0x00,
            0xff, 0x00, 0x00,
            0xff, 0xff, 0xff,

            0x00, 0x00, 0x00, 0x01,
            0x7f, 0xff, 0xff, 0xff,
            0x80, 0x00, 0x00, 0x00,
            0xff, 0xff, 0xff, 0xff,
        ][..]);

        assert_eq!(reader.read_1_byte_signed().unwrap(), 1);
        assert_eq!(reader.read_1_byte_signed().unwrap(), 127);
        assert_eq!(reader.read_1_byte_signed().unwrap(), -128);
        assert_eq!(reader.read_1_byte_signed().unwrap(), -1);

        assert_eq!(reader.read_2_bytes_signed().unwrap(), 1);
        assert_eq!(reader.read_2_bytes_signed().unwrap(), 32767);
        assert_eq!(reader.read_2_bytes_signed().unwrap(), -32768);
        assert_eq!(reader.read_2_bytes_signed().unwrap(), -1);

        assert_eq!(reader.read_3_bytes_signed().unwrap(), 1);
        assert_eq!(reader.read_3_bytes_signed().unwrap(), 8388607);
        assert_eq!(reader.read_3_bytes_signed().unwrap(), -8388608);
        assert_eq!(reader.read_3_bytes_signed().unwrap(), -65536);
        assert_eq!(reader.read_3_bytes_signed().unwrap(), -1);

        assert_eq!(reader.read_4_bytes_signed().unwrap(), 1);
        assert_eq!(reader.read_4_bytes_signed().unwrap(), 2147483647);
        assert_eq!(reader.read_4_bytes_signed().unwrap(), -2147483648);
        assert_eq!(reader.read_4_bytes_signed().unwrap(), -1);
    }

    #[test]
    fn it_reads_arrays() {
        let mut reader = DVIFileReader::new(&[0x01, 0x02, 0x03, 0x04][..]);

        assert_eq!(reader.read_array(4).unwrap(), vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn it_reads_strings() {
        let mut reader = DVIFileReader::new(
            &[
                0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72,
                0x6c, 0x64, 0x21,
            ][..],
        );

        assert_eq!(reader.read_string(13).unwrap(), "Hello, world!");
    }
}

/// Methods for writing DVI files
use std::io;

use super::file::{DVICommand, DVIFile};

impl DVIFile {
    pub fn write_to<T: io::Write>(&self, output: T) -> io::Result<()> {
        let mut writer = DVIFileWriter::new(output);

        for command in &self.commands {
            writer.write(command)?;
        }

        Ok(())
    }
}

pub struct DVIFileWriter<T: io::Write> {
    writer: T,
}

macro_rules! generate_int_writer_func {
    (fn $func_name:ident($input_type:ident, $from:expr, $to:expr)) =>
    {
        fn $func_name(&mut self, value: $input_type) -> io::Result<()> {
            self.writer.write_all(&value.to_be_bytes()[$from..$to])
        }
    }
}

impl<T: io::Write> DVIFileWriter<T> {
    pub fn new(writer: T) -> Self {
        DVIFileWriter { writer: writer }
    }

    generate_int_writer_func!(fn write_1_byte_unsigned(u8, 0, 1));
    generate_int_writer_func!(fn write_2_bytes_unsigned(u16, 0, 2));
    generate_int_writer_func!(fn write_3_bytes_unsigned(u32, 1, 4));
    generate_int_writer_func!(fn write_4_bytes_unsigned(u32, 0, 4));

    generate_int_writer_func!(fn write_1_byte_signed(i8, 0, 1));
    generate_int_writer_func!(fn write_2_bytes_signed(i16, 0, 2));
    generate_int_writer_func!(fn write_3_bytes_signed(i32, 1, 4));
    generate_int_writer_func!(fn write_4_bytes_signed(i32, 0, 4));

    fn write_array(&mut self, value: &[u8], size: usize) -> io::Result<()> {
        if value.len() < size {
            self.writer.write_all(&value)?;
            for i in 0..(size - value.len()) {
                self.writer.write_all(&[0])?;
            }
        } else {
            self.writer.write_all(&value[0..size])?;
        }

        Ok(())
    }

    fn write_string(&mut self, value: &str, size: usize) -> io::Result<()> {
        self.write_array(value.as_bytes(), size)
    }

    fn write(&mut self, command: &DVICommand) -> io::Result<()> {
        // TODO(xymostech): Write out all of the commands
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_writes_unsigned_numbers() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = DVIFileWriter::new(&mut output);

        writer.write_1_byte_unsigned(1).unwrap();
        writer.write_1_byte_unsigned(255).unwrap();

        writer.write_2_bytes_unsigned(1).unwrap();
        writer.write_2_bytes_unsigned(256).unwrap();
        writer.write_2_bytes_unsigned(65535).unwrap();

        writer.write_3_bytes_unsigned(1).unwrap();
        writer.write_3_bytes_unsigned(256).unwrap();
        writer.write_3_bytes_unsigned(65536).unwrap();
        writer.write_3_bytes_unsigned(16777215).unwrap();

        writer.write_4_bytes_unsigned(1).unwrap();
        writer.write_4_bytes_unsigned(256).unwrap();
        writer.write_4_bytes_unsigned(65536).unwrap();
        writer.write_4_bytes_unsigned(16777216).unwrap();
        writer.write_4_bytes_unsigned(4294967295).unwrap();

        #[rustfmt::skip]
        assert_eq!(
            output,
            vec![
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
            ]);
    }

    #[test]
    fn it_writes_signed_numbers() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = DVIFileWriter::new(&mut output);

        writer.write_1_byte_signed(1).unwrap();
        writer.write_1_byte_signed(127).unwrap();
        writer.write_1_byte_signed(-128).unwrap();
        writer.write_1_byte_signed(-1).unwrap();

        writer.write_2_bytes_signed(1).unwrap();
        writer.write_2_bytes_signed(32767).unwrap();
        writer.write_2_bytes_signed(-32768).unwrap();
        writer.write_2_bytes_signed(-1).unwrap();

        writer.write_3_bytes_signed(1).unwrap();
        writer.write_3_bytes_signed(8388607).unwrap();
        writer.write_3_bytes_signed(-8388608).unwrap();
        writer.write_3_bytes_signed(-65536).unwrap();
        writer.write_3_bytes_signed(-1).unwrap();

        writer.write_4_bytes_signed(1).unwrap();
        writer.write_4_bytes_signed(2147483647).unwrap();
        writer.write_4_bytes_signed(-2147483648).unwrap();
        writer.write_4_bytes_signed(-1).unwrap();

        #[rustfmt::skip]
        assert_eq!(
            output,
            vec![
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
            ]);
    }

    #[test]
    fn it_writes_arrays() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = DVIFileWriter::new(&mut output);

        writer.write_array(&[0x00, 0x01, 0x02], 2).unwrap();
        writer.write_array(&[0x00, 0x01, 0x02], 4).unwrap();

        assert_eq!(output, vec![0x00, 0x01, 0x00, 0x01, 0x02, 0x00,]);
    }

    #[test]
    fn it_writes_strings() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = DVIFileWriter::new(&mut output);

        writer.write_string("hello", 3).unwrap();
        writer.write_string("hello", 6).unwrap();

        assert_eq!(
            output,
            vec![
                'h' as u8, 'e' as u8, 'l' as u8, 'h' as u8, 'e' as u8,
                'l' as u8, 'l' as u8, 'o' as u8, 0x00,
            ]
        );
    }
}

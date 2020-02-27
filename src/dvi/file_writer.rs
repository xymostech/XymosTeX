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
            for _ in 0..(size - value.len()) {
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
        command.write_to(self)
    }
}

impl DVICommand {
    fn write_to<T: io::Write>(
        &self,
        writer: &mut DVIFileWriter<T>,
    ) -> io::Result<()> {
        match self {
            DVICommand::SetCharN(n) => writer.write_1_byte_unsigned(*n),
            DVICommand::Bop { cs, pointer } => {
                writer.write_1_byte_unsigned(139)?;
                for c in cs {
                    writer.write_4_bytes_signed(*c)?;
                }
                writer.write_4_bytes_signed(*pointer)
            }
            DVICommand::Eop => writer.write_1_byte_unsigned(140),
            DVICommand::Push => writer.write_1_byte_unsigned(141),
            DVICommand::Pop => writer.write_1_byte_unsigned(142),
            DVICommand::Right2(b) => {
                writer.write_1_byte_unsigned(144)?;
                writer.write_2_bytes_signed(*b)
            }
            DVICommand::Right3(b) => {
                writer.write_1_byte_unsigned(145)?;
                writer.write_3_bytes_signed(*b)
            }
            DVICommand::Right4(b) => {
                writer.write_1_byte_unsigned(146)?;
                writer.write_4_bytes_signed(*b)
            }
            DVICommand::W0 => writer.write_1_byte_unsigned(147),
            DVICommand::W2(b) => {
                writer.write_1_byte_unsigned(149)?;
                writer.write_2_bytes_signed(*b)
            }
            DVICommand::W3(b) => {
                writer.write_1_byte_unsigned(150)?;
                writer.write_3_bytes_signed(*b)
            }
            DVICommand::Down2(a) => {
                writer.write_1_byte_unsigned(158)?;
                writer.write_2_bytes_signed(*a)
            }
            DVICommand::Down3(a) => {
                writer.write_1_byte_unsigned(159)?;
                writer.write_3_bytes_signed(*a)
            }
            DVICommand::Down4(a) => {
                writer.write_1_byte_unsigned(160)?;
                writer.write_4_bytes_signed(*a)
            }
            DVICommand::Y0 => writer.write_1_byte_unsigned(161),
            DVICommand::Y3(a) => {
                writer.write_1_byte_unsigned(164)?;
                writer.write_3_bytes_signed(*a)
            }
            DVICommand::FntNumN(n) => writer.write_1_byte_unsigned(n + 171),
            DVICommand::Fnt4(n) => {
                writer.write_1_byte_unsigned(238)?;
                writer.write_4_bytes_signed(*n)
            }
            DVICommand::FntDef1 {
                font_num,
                checksum,
                scale,
                design_size,
                area,
                length,
                font_name,
            } => {
                writer.write_1_byte_unsigned(243)?;
                writer.write_1_byte_unsigned(*font_num)?;
                writer.write_4_bytes_unsigned(*checksum)?;
                writer.write_4_bytes_unsigned(*scale)?;
                writer.write_4_bytes_unsigned(*design_size)?;
                writer.write_1_byte_unsigned(*area)?;
                writer.write_1_byte_unsigned(*length)?;
                writer.write_string(&font_name, (area + length) as usize)
            }
            DVICommand::FntDef4 {
                font_num,
                checksum,
                scale,
                design_size,
                area,
                length,
                font_name,
            } => {
                writer.write_1_byte_unsigned(246)?;
                writer.write_4_bytes_signed(*font_num)?;
                writer.write_4_bytes_unsigned(*checksum)?;
                writer.write_4_bytes_unsigned(*scale)?;
                writer.write_4_bytes_unsigned(*design_size)?;
                writer.write_1_byte_unsigned(*area)?;
                writer.write_1_byte_unsigned(*length)?;
                writer.write_string(&font_name, (area + length) as usize)
            }
            DVICommand::Pre {
                format,
                num,
                den,
                mag,
                comment,
            } => {
                writer.write_1_byte_unsigned(247)?;
                writer.write_1_byte_unsigned(*format)?;
                writer.write_4_bytes_unsigned(*num)?;
                writer.write_4_bytes_unsigned(*den)?;
                writer.write_4_bytes_unsigned(*mag)?;
                writer.write_1_byte_unsigned(comment.len() as u8)?;
                writer.write_array(&comment, comment.len())
            }
            DVICommand::Post {
                pointer,
                num,
                den,
                mag,
                max_page_height,
                max_page_width,
                max_stack_depth,
                num_pages,
            } => {
                writer.write_1_byte_unsigned(248)?;
                writer.write_4_bytes_unsigned(*pointer)?;
                writer.write_4_bytes_unsigned(*num)?;
                writer.write_4_bytes_unsigned(*den)?;
                writer.write_4_bytes_unsigned(*mag)?;
                writer.write_4_bytes_unsigned(*max_page_height)?;
                writer.write_4_bytes_unsigned(*max_page_width)?;
                writer.write_2_bytes_unsigned(*max_stack_depth)?;
                writer.write_2_bytes_unsigned(*num_pages)
            }
            DVICommand::PostPost {
                post_pointer,
                format,
                tail,
            } => {
                writer.write_1_byte_unsigned(249)?;
                writer.write_4_bytes_unsigned(*post_pointer)?;
                writer.write_1_byte_unsigned(*format)?;

                for _ in 0..*tail {
                    writer.write_1_byte_unsigned(223)?;
                }

                Ok(())
            }
            command => {
                panic!("Unimplemented command: {:?}", command);
            }
        }
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

    #[test]
    fn it_writes_commands() {
        // TODO(xymostech): Move this test case into a shared file because it's
        // currently copied between here and the parser tests.
        let file = DVIFile {
            commands: vec![
                DVICommand::Pre {
                    format: 2,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    comment: vec!['h' as u8, 'i' as u8],
                },
                DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                },
                DVICommand::Push,
                DVICommand::Pop,
                DVICommand::FntDef1 {
                    font_num: 0,
                    checksum: 305419896,
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 5,
                    font_name: "cmr10".to_string(),
                },
                DVICommand::FntNumN(63),
                DVICommand::FntNumN(0),
                DVICommand::SetCharN(0),
                DVICommand::SetCharN(127),
                DVICommand::Right2(-30000),
                DVICommand::Right3(-60000),
                DVICommand::Right4(-123456789),
                DVICommand::W0,
                DVICommand::W2(-30000),
                DVICommand::W3(-60000),
                DVICommand::Down2(-30000),
                DVICommand::Down3(-60000),
                DVICommand::Down4(-123456789),
                DVICommand::Y0,
                DVICommand::Y3(-60000),
                DVICommand::Eop,
                DVICommand::Post {
                    pointer: 18,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    max_page_width: 65536,
                    max_page_height: 65536,
                    max_stack_depth: 1,
                    num_pages: 1,
                },
                DVICommand::FntDef1 {
                    font_num: 0,
                    checksum: 305419896,
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 5,
                    font_name: "cmr10".to_string(),
                },
                DVICommand::PostPost {
                    post_pointer: 128,
                    format: 2,
                    tail: 6,
                },
            ],
        };

        let mut output: Vec<u8> = Vec::new();
        file.write_to(&mut output).unwrap();

        #[rustfmt::skip]
        assert_eq!(
            output,
            vec![
                // pre
                247,
                2,
                1, 131, 146, 192,
                28, 59, 0, 0,
                0, 0, 3, 232,
                2, 'h' as u8, 'i' as u8,

                // bop
                139,
                0, 0, 0, 1,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0, 0, 0, 0,
                0xff, 0xff, 0xff, 0xff,

                // push
                141,

                // pop
                142,

                // fnt_def1
                243,
                0,
                0x12, 0x34, 0x56, 0x78,
                0, 10, 0, 0,
                0, 10, 0, 0,
                0,
                5,
                'c' as u8, 'm' as u8, 'r' as u8, '1' as u8, '0' as u8,

                // fnt_num 63
                234,

                // fnt_num 0
                171,

                // set_char 0
                0,

                // set_char 127
                127,

                // right2
                144, 138, 208,

                // right3
                145, 255, 21, 160,

                // right4
                146, 248, 164, 50, 235,

                // w0
                147,

                // w2
                149, 138, 208,

                // w3
                150, 255, 21, 160,

                // down2
                158, 138, 208,

                // down3
                159, 255, 21, 160,

                // down4
                160, 248, 164, 50, 235,

                // y0
                161,

                // y3
                164, 255, 21, 160,

                // eop
                140,

                // post
                248,
                0, 0, 0, 18,
                1, 131, 146, 192,
                28, 59, 0, 0,
                0, 0, 3, 232,
                0, 1, 0, 0,
                0, 1, 0, 0,
                0, 1,
                0, 1,

                // fnt_def1
                243,
                0,
                0x12, 0x34, 0x56, 0x78,
                0, 10, 0, 0,
                0, 10, 0, 0,
                0,
                5,
                'c' as u8, 'm' as u8, 'r' as u8, '1' as u8, '0' as u8,

                // post_post
                249,
                0, 0, 0, 128,
                2,
                223, 223, 223, 223, 223, 223,
            ]);
    }
}

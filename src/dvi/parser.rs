/// Methods for parsing DVI files
use std::io;

use crate::dvi::file::{DVICommand, DVIFile};
use crate::dvi::file_reader::DVIFileReader;

impl DVICommand {
    /// Read a single DVI command from a file
    fn read_from<T: io::Read>(
        reader: &mut DVIFileReader<T>,
    ) -> io::Result<Option<Self>> {
        // Read the opcode. If this fails with UnexpectedEof, we're at the end
        // of the file, so there are no more commands and we return None.
        // Everywhere else, we just return the error.
        let opcode = match reader.read_1_byte_unsigned() {
            Ok(opcode) => opcode,
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok(None);
                } else {
                    return Err(err);
                }
            }
        };
        match opcode {
            // set_char_n
            n if n <= 127 => Ok(Some(DVICommand::SetCharN(n))),
            // bop
            139 => {
                let mut cs = [0; 10];
                for i in 0..10 {
                    cs[i] = reader.read_4_bytes_signed()?;
                }
                let p = reader.read_4_bytes_signed()?;

                Ok(Some(DVICommand::Bop { cs: cs, pointer: p }))
            }
            // eop
            140 => Ok(Some(DVICommand::Eop)),
            // push
            141 => Ok(Some(DVICommand::Push)),
            // pop
            142 => Ok(Some(DVICommand::Pop)),
            // right2
            144 => {
                let b = reader.read_2_bytes_signed()?;
                Ok(Some(DVICommand::Right2(b)))
            }
            // right3
            145 => {
                let b = reader.read_3_bytes_signed()?;
                Ok(Some(DVICommand::Right3(b)))
            }
            // right4
            146 => {
                let b = reader.read_4_bytes_signed()?;
                Ok(Some(DVICommand::Right4(b)))
            }
            // w0
            147 => Ok(Some(DVICommand::W0)),
            // w2
            149 => {
                let b = reader.read_2_bytes_signed()?;
                Ok(Some(DVICommand::W2(b)))
            }
            // w3
            150 => {
                let b = reader.read_3_bytes_signed()?;
                Ok(Some(DVICommand::W3(b)))
            }
            // x0
            152 => Ok(Some(DVICommand::X0)),
            // x2
            154 => {
                let b = reader.read_2_bytes_signed()?;
                Ok(Some(DVICommand::X2(b)))
            }
            // x3
            155 => {
                let b = reader.read_3_bytes_signed()?;
                Ok(Some(DVICommand::X3(b)))
            }
            // down2
            158 => {
                let a = reader.read_2_bytes_signed()?;
                Ok(Some(DVICommand::Down2(a)))
            }
            // down3
            159 => {
                let a = reader.read_3_bytes_signed()?;
                Ok(Some(DVICommand::Down3(a)))
            }
            // down4
            160 => {
                let a = reader.read_4_bytes_signed()?;
                Ok(Some(DVICommand::Down4(a)))
            }
            // y0
            161 => Ok(Some(DVICommand::Y0)),
            // y3
            164 => {
                let a = reader.read_3_bytes_signed()?;
                Ok(Some(DVICommand::Y3(a)))
            }
            // fnt_num_n
            n if n >= 171 && n <= 234 => Ok(Some(DVICommand::FntNumN(n - 171))),
            // fnt4
            238 => {
                let k = reader.read_4_bytes_signed()?;
                Ok(Some(DVICommand::Fnt4(k)))
            }
            // fnt_def1
            243 => {
                let k = reader.read_1_byte_unsigned()?;
                let c = reader.read_4_bytes_unsigned()?;
                let s = reader.read_4_bytes_unsigned()?;
                let d = reader.read_4_bytes_unsigned()?;
                let a = reader.read_1_byte_unsigned()?;
                let l = reader.read_1_byte_unsigned()?;
                let n = reader.read_string((a + l) as usize)?;
                Ok(Some(DVICommand::FntDef1 {
                    font_num: k,
                    checksum: c,
                    scale: s,
                    design_size: d,
                    area: a,
                    length: l,
                    font_name: n,
                }))
            }
            // fnt_def4
            246 => {
                let k = reader.read_4_bytes_signed()?;
                let c = reader.read_4_bytes_unsigned()?;
                let s = reader.read_4_bytes_unsigned()?;
                let d = reader.read_4_bytes_unsigned()?;
                let a = reader.read_1_byte_unsigned()?;
                let l = reader.read_1_byte_unsigned()?;
                let n = reader.read_string((a + l) as usize)?;
                Ok(Some(DVICommand::FntDef4 {
                    font_num: k,
                    checksum: c,
                    scale: s,
                    design_size: d,
                    area: a,
                    length: l,
                    font_name: n,
                }))
            }
            // pre
            247 => {
                let i = reader.read_1_byte_unsigned()?;
                assert!(i == 2, "Unknown DVI format: {}", i);
                let num = reader.read_4_bytes_unsigned()?;
                let den = reader.read_4_bytes_unsigned()?;
                let mag = reader.read_4_bytes_unsigned()?;
                let k = reader.read_1_byte_unsigned()?;
                let x = reader.read_array(k as usize)?;

                Ok(Some(DVICommand::Pre {
                    format: i,
                    num: num,
                    den: den,
                    mag: mag,
                    comment: x,
                }))
            }
            // post
            248 => {
                let p = reader.read_4_bytes_unsigned()?;
                let num = reader.read_4_bytes_unsigned()?;
                let den = reader.read_4_bytes_unsigned()?;
                let mag = reader.read_4_bytes_unsigned()?;
                let l = reader.read_4_bytes_unsigned()?;
                let u = reader.read_4_bytes_unsigned()?;
                let s = reader.read_2_bytes_unsigned()?;
                let t = reader.read_2_bytes_unsigned()?;

                Ok(Some(DVICommand::Post {
                    pointer: p,
                    num: num,
                    den: den,
                    mag: mag,
                    max_page_height: l,
                    max_page_width: u,
                    max_stack_depth: s,
                    num_pages: t,
                }))
            }
            // post_post
            249 => {
                let q = reader.read_4_bytes_unsigned()?;
                let i = reader.read_1_byte_unsigned()?;

                let mut num_223s = 0;
                while let Ok(_) = reader.read_1_byte_unsigned() {
                    num_223s += 1;
                }

                Ok(Some(DVICommand::PostPost {
                    post_pointer: q,
                    format: i,
                    tail: num_223s,
                }))
            }
            n if n <= 249 => panic!("Unimplemented opcode: {}", n),
            n => panic!("Invalid opcode: {}", n),
        }
    }
}

impl DVIFile {
    /// Read a DVIFile from a file
    pub fn new<T: io::Read>(reader: T) -> io::Result<Self> {
        let mut file_reader = DVIFileReader::new(reader);

        let mut commands = Vec::new();
        while let Some(command) = DVICommand::read_from(&mut file_reader)? {
            commands.push(command);
        }
        Ok(DVIFile { commands: commands })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_DVI: &[u8] = include_bytes!("test_files/test.dvi");

    #[test]
    fn it_parses_dvis() {
        #[rustfmt::skip]
        let file = DVIFile::new(
            &[
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
            ][..],
        )
        .unwrap();

        assert_eq!(
            file,
            DVIFile {
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
            }
        )
    }

    #[test]
    fn it_parses_test_file() {
        let file = DVIFile::new(TEST_DVI).unwrap();
        assert_eq!(file.commands.len(), 148);
    }
}

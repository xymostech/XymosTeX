use std::io;

use crate::tfm::file_reader::TeXFileReader;
use crate::tfm::*;

impl TFMFile {
    pub fn new<T: io::Read>(reader: T) -> io::Result<TFMFile> {
        let mut file_reader = TeXFileReader::new(reader);

        let file_length = file_reader.read_16bit_int()?;
        let header_length = file_reader.read_16bit_int()?;
        let first_char = file_reader.read_16bit_int()?;
        let last_char = file_reader.read_16bit_int()?;
        let num_widths = file_reader.read_16bit_int()?;
        let num_heights = file_reader.read_16bit_int()?;
        let num_depths = file_reader.read_16bit_int()?;
        let num_italic_corrections = file_reader.read_16bit_int()?;
        let num_lig_kerns = file_reader.read_16bit_int()?;
        let num_kerns = file_reader.read_16bit_int()?;
        let num_ext_recipes = file_reader.read_16bit_int()?;
        let num_params = file_reader.read_16bit_int()?;

        assert!(
            header_length == 18,
            "Invalid header length. Only 18 word headers are \
             supported"
        );
        assert!(
            file_length
                == 6 + header_length
                    + (last_char - first_char + 1)
                    + num_widths
                    + num_heights
                    + num_depths
                    + num_italic_corrections
                    + num_lig_kerns
                    + num_kerns
                    + num_ext_recipes
                    + num_params,
            "Invalid length values. The sum of other lengths must total to \
             the file length"
        );

        let header = TFMFile::read_header(&mut file_reader)?;

        let char_infos: Vec<CharInfoEntry> = (first_char..last_char + 1)
            .map(|_| TFMFile::read_char_info(&mut file_reader))
            .collect::<io::Result<Vec<_>>>()?;

        let widths = TFMFile::read_n_fixnums(&mut file_reader, num_widths)?;
        let heights = TFMFile::read_n_fixnums(&mut file_reader, num_heights)?;
        let depths = TFMFile::read_n_fixnums(&mut file_reader, num_depths)?;
        let italic_corrections =
            TFMFile::read_n_fixnums(&mut file_reader, num_italic_corrections)?;

        let lig_kern_steps = (0..num_lig_kerns)
            .map(|_| TFMFile::read_lig_kern_step(&mut file_reader))
            .collect::<io::Result<Vec<_>>>()?;

        let kerns = TFMFile::read_n_fixnums(&mut file_reader, num_kerns)?;

        let ext_recipes = (0..num_ext_recipes)
            .map(|_| TFMFile::read_extensible_recipe(&mut file_reader))
            .collect::<io::Result<Vec<_>>>()?;

        let font_parameters =
            TFMFile::read_n_fixnums(&mut file_reader, num_params)?;

        Ok(TFMFile {
            first_char: first_char as usize,
            last_char: last_char as usize,

            header: header,

            char_infos: char_infos,
            widths: widths,
            heights: heights,
            depths: depths,
            italic_corrections: italic_corrections,
            lig_kern_steps: lig_kern_steps,
            kerns: kerns,
            ext_recipes: ext_recipes,
            font_parameters: font_parameters,
        })
    }

    fn read_header<T: io::Read>(
        file_reader: &mut TeXFileReader<T>,
    ) -> io::Result<TFMHeader> {
        let checksum = file_reader.read_32bit_int()?;
        let design_size = file_reader.read_fixnum()?;
        let coding_scheme = file_reader.read_string(40)?;
        let parc_font_identifier = file_reader.read_string(20)?;
        let seven_bit_safe = file_reader.read_8bit_int()? == 0b1000_0000;
        let _unused = file_reader.read_16bit_int()?;
        let parc_face_byte = file_reader.read_8bit_int()?;

        Ok(TFMHeader {
            checksum: checksum,
            design_size: design_size,
            coding_scheme: coding_scheme,
            parc_font_identifier: parc_font_identifier,
            seven_bit_safe: seven_bit_safe,
            parc_face_byte: parc_face_byte,
        })
    }

    fn read_char_info<T: io::Read>(
        file_reader: &mut TeXFileReader<T>,
    ) -> io::Result<CharInfoEntry> {
        let width_index = file_reader.read_8bit_int()?;

        let height_and_depth = file_reader.read_8bit_int()?;
        let height_index = (height_and_depth & 0b1111_0000) >> 4;
        let depth_index = height_and_depth & 0b0000_1111;

        let ic_and_tag = file_reader.read_8bit_int()?;
        let italic_correction_index = (ic_and_tag & 0b1111_1100) >> 2;
        let tag = ic_and_tag & 0b0000_0011;

        let remainder = file_reader.read_8bit_int()?;

        let kind = match tag {
            0 => CharKind::Vanilla,
            1 => CharKind::LigKern {
                ligkern_index: remainder as usize,
            },
            2 => CharKind::CharList {
                next_char: remainder as usize,
            },
            3 => CharKind::Extensible {
                ext_recipe_index: remainder as usize,
            },
            _ => unreachable!(),
        };

        Ok(CharInfoEntry {
            width_index: width_index as usize,
            height_index: height_index as usize,
            depth_index: depth_index as usize,
            italic_correction_index: italic_correction_index as usize,
            kind: kind,
        })
    }

    fn read_n_fixnums<T: io::Read>(
        file_reader: &mut TeXFileReader<T>,
        num: u16,
    ) -> io::Result<Vec<f64>> {
        (0..num).map(|_| file_reader.read_fixnum()).collect()
    }

    fn read_lig_kern_step<T: io::Read>(
        file_reader: &mut TeXFileReader<T>,
    ) -> io::Result<LigKernStep> {
        let stop = file_reader.read_8bit_int()? == 0b1000_0000;
        let next_char = file_reader.read_8bit_int()?;
        let tag = file_reader.read_8bit_int()?;
        let remainder = file_reader.read_8bit_int()?;

        let kind = match tag {
            0b0000_0000 => LigKernKind::Ligature {
                substitution: remainder as usize,
            },
            0b1000_0000 => LigKernKind::Kern {
                kern_index: remainder as usize,
            },
            _ => panic!("Invalid tag byte: {}", tag),
        };

        Ok(LigKernStep {
            stop: stop,
            next_char: next_char as usize,
            kind: kind,
        })
    }

    fn read_extensible_recipe<T: io::Read>(
        file_reader: &mut TeXFileReader<T>,
    ) -> io::Result<ExtRecipe> {
        let top = file_reader.read_8bit_int()? as usize;
        let mid = file_reader.read_8bit_int()? as usize;
        let bot = file_reader.read_8bit_int()? as usize;
        let ext = file_reader.read_8bit_int()? as usize;

        Ok(ExtRecipe {
            top: top,
            mid: mid,
            bot: bot,
            ext: ext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tfm::test_data::BASIC_TFM;

    #[test]
    fn it_reads_basics() {
        let file = TFMFile::new(&BASIC_TFM[..]).unwrap();

        assert_eq!(
            file,
            TFMFile {
                first_char: 'a' as usize,
                last_char: 'a' as usize,

                header: TFMHeader {
                    checksum: 0xABCDEFAB,
                    design_size: 10.0,
                    coding_scheme: "testing".to_string(),
                    parc_font_identifier: "hi parc".to_string(),
                    seven_bit_safe: true,
                    parc_face_byte: 0xab,
                },

                char_infos: vec![CharInfoEntry {
                    width_index: 1,
                    height_index: 1,
                    depth_index: 1,
                    italic_correction_index: 1,
                    kind: CharKind::Vanilla,
                }],

                widths: vec![0.0, 3.5],
                heights: vec![0.0, 5.5],
                depths: vec![0.0, 0.5],
                italic_corrections: vec![0.0, 0.25],
                lig_kern_steps: vec![],
                kerns: vec![],
                ext_recipes: vec![],
                font_parameters: vec![0.0, 4.0, 1.0, 2.0, 5.5, 4.0, 1.0,],
            }
        );
    }
}

use crate::dimension::{Dimen, Unit};
use crate::tfm::fixnum::Fixnum;
use crate::tfm::{CharInfoEntry, CharKind, TFMFile};

impl TFMFile {
    pub fn get_design_size(&self) -> f64 {
        self.header.design_size.as_float()
    }

    fn get_char_info(&self, chr: char) -> &CharInfoEntry {
        let char_index = chr as usize;
        assert!(
            self.first_char <= char_index && char_index <= self.last_char,
            "Char not found in font: {}",
            chr
        );

        let char_info_index = char_index - self.first_char;
        &self.char_infos[char_info_index]
    }

    fn get_dimen_relative_to_scale_or_design_size(
        &self,
        dimen: &Fixnum,
        scale: Option<&Dimen>,
    ) -> Dimen {
        let dimen_ratio = dimen.as_ratio();

        if let Some(scale) = scale {
            Dimen::from_scaled_points(
                ((scale.as_scaled_points() as i64) * dimen_ratio.0
                    / dimen_ratio.1) as i32,
            )
        } else {
            let design_size = self.header.design_size.as_ratio();
            Dimen::from_scaled_points(
                (design_size.0 * dimen_ratio.0
                    / dimen_ratio.1
                    / (design_size.1 / 65536)) as i32,
            )
        }
    }

    pub fn get_width(&self, chr: char, scale: Option<&Dimen>) -> Dimen {
        let char_info = self.get_char_info(chr);
        self.get_dimen_relative_to_scale_or_design_size(
            &self.widths[char_info.width_index],
            scale,
        )
    }

    pub fn get_height(&self, chr: char, scale: Option<&Dimen>) -> Dimen {
        let char_info = self.get_char_info(chr);
        self.get_dimen_relative_to_scale_or_design_size(
            &self.heights[char_info.height_index],
            scale,
        )
    }

    pub fn get_depth(&self, chr: char, scale: Option<&Dimen>) -> Dimen {
        let char_info = self.get_char_info(chr);
        self.get_dimen_relative_to_scale_or_design_size(
            &self.depths[char_info.depth_index],
            scale,
        )
    }

    pub const fn get_checksum(&self) -> u32 {
        self.header.checksum
    }

    pub fn get_font_dimension(
        &self,
        dimen_number: usize,
        scale: Option<&Dimen>,
    ) -> Dimen {
        self.get_dimen_relative_to_scale_or_design_size(
            &self.font_parameters[dimen_number - 1],
            scale,
        )
    }

    pub fn get_successor(&self, chr: char) -> char {
        let char_info = self.get_char_info(chr);

        match char_info.kind {
            CharKind::CharList { next_char } => next_char as char,
            _ => chr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tfm::test_data::{BASIC_TFM, CMR10_TFM};

    #[test]
    fn get_basic_dimensions() {
        let font_metrics = TFMFile::new(&BASIC_TFM[..]).unwrap();

        assert_eq!(
            font_metrics.get_width('a', None),
            Dimen::from_unit(17.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_height('a', None),
            Dimen::from_unit(27.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_depth('a', None),
            Dimen::from_unit(2.5, Unit::Point)
        );
    }

    #[test]
    fn get_cmr10_dimensions() {
        let font_metrics = TFMFile::new(CMR10_TFM).unwrap();

        assert_eq!(
            font_metrics.get_width('a', None),
            Dimen::from_scaled_points(327681)
        );
        assert!(font_metrics.get_height('a', None) > Dimen::zero());
        assert!(
            font_metrics.get_height('t', None)
                > font_metrics.get_height('a', None)
        );
        assert!(font_metrics.get_depth('g', None) > Dimen::zero());
        assert!(
            font_metrics.get_width('w', None)
                > font_metrics.get_width('i', None)
        );

        for ch in (0 as u8)..128 {
            assert!(font_metrics.get_width(ch as char, None) > Dimen::zero());
        }
    }

    #[test]
    fn get_cmr10_font_dimens() {
        let font_metrics = TFMFile::new(CMR10_TFM).unwrap();

        assert_eq!(font_metrics.get_font_dimension(1, None), Dimen::zero());
        assert_eq!(
            font_metrics.get_font_dimension(2, None),
            Dimen::from_scaled_points(218453)
        );
        assert_eq!(
            font_metrics.get_font_dimension(3, None),
            Dimen::from_scaled_points(109226)
        );
        assert_eq!(
            font_metrics.get_font_dimension(4, None),
            Dimen::from_scaled_points(72818)
        );
        assert_eq!(
            font_metrics.get_font_dimension(5, None),
            Dimen::from_scaled_points(282168)
        );
        assert_eq!(
            font_metrics.get_font_dimension(6, None),
            Dimen::from_scaled_points(655361)
        );
        assert_eq!(
            font_metrics.get_font_dimension(7, None),
            Dimen::from_scaled_points(72818)
        );
    }
}

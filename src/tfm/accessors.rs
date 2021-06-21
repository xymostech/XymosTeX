use crate::dimension::{Dimen, Unit};
use crate::tfm::{CharInfoEntry, TFMFile};

impl TFMFile {
    pub fn get_design_size(&self) -> f64 {
        self.header.design_size
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

    pub fn get_width(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(
            self.header.design_size * self.widths[char_info.width_index],
            Unit::Point,
        )
    }

    pub fn get_height(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(
            self.header.design_size * self.heights[char_info.height_index],
            Unit::Point,
        )
    }

    pub fn get_depth(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(
            self.header.design_size * self.depths[char_info.depth_index],
            Unit::Point,
        )
    }

    pub const fn get_checksum(&self) -> u32 {
        self.header.checksum
    }

    pub fn get_font_dimension(&self, dimen_number: usize) -> Dimen {
        Dimen::from_unit(
            self.header.design_size * self.font_parameters[dimen_number - 1],
            Unit::Point,
        )
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
            font_metrics.get_width('a'),
            Dimen::from_unit(17.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_height('a'),
            Dimen::from_unit(27.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_depth('a'),
            Dimen::from_unit(2.5, Unit::Point)
        );
    }

    #[test]
    fn get_cmr10_dimensions() {
        let font_metrics = TFMFile::new(CMR10_TFM).unwrap();

        assert_eq!(
            font_metrics.get_width('a'),
            Dimen::from_scaled_points(327681)
        );
        assert!(font_metrics.get_height('a') > Dimen::zero());
        assert!(font_metrics.get_height('t') > font_metrics.get_height('a'));
        assert!(font_metrics.get_depth('g') > Dimen::zero());
        assert!(font_metrics.get_width('w') > font_metrics.get_width('i'));

        for ch in (0 as u8)..128 {
            assert!(font_metrics.get_width(ch as char) > Dimen::zero());
        }
    }

    #[test]
    fn get_cmr10_font_dimens() {
        let font_metrics = TFMFile::new(CMR10_TFM).unwrap();

        assert_eq!(font_metrics.get_font_dimension(1), Dimen::zero());
        assert_eq!(
            font_metrics.get_font_dimension(2),
            Dimen::from_scaled_points(218453)
        );
        assert_eq!(
            font_metrics.get_font_dimension(3),
            Dimen::from_scaled_points(109226)
        );
        assert_eq!(
            font_metrics.get_font_dimension(4),
            Dimen::from_scaled_points(72818)
        );
        assert_eq!(
            font_metrics.get_font_dimension(5),
            Dimen::from_scaled_points(282168)
        );
        assert_eq!(
            font_metrics.get_font_dimension(6),
            Dimen::from_scaled_points(655361)
        );
        assert_eq!(
            font_metrics.get_font_dimension(7),
            Dimen::from_scaled_points(72818)
        );
    }
}

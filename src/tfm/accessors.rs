use crate::dimension::{Dimen, Unit};
use crate::tfm::test_data::BASIC_TFM;
use crate::tfm::{CharInfoEntry, TFMFile};

impl TFMFile {
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

    fn get_width(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(self.widths[char_info.width_index], Unit::Point)
    }

    fn get_height(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(self.heights[char_info.width_index], Unit::Point)
    }

    fn get_depth(&self, chr: char) -> Dimen {
        let char_info = self.get_char_info(chr);

        Dimen::from_unit(self.depths[char_info.width_index], Unit::Point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_basic_dimensions() {
        let font_metrics = TFMFile::new(&BASIC_TFM[..]).unwrap();

        assert_eq!(
            font_metrics.get_width('a'),
            Dimen::from_unit(3.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_height('a'),
            Dimen::from_unit(5.5, Unit::Point)
        );
        assert_eq!(
            font_metrics.get_depth('a'),
            Dimen::from_unit(0.5, Unit::Point)
        );
    }
}

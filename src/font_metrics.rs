use crate::dimension::Dimen;
use crate::font::Font;
use crate::paths::get_path_to_font;
use crate::tfm::TFMFile;

#[derive(Debug)]
pub struct FontMetrics {
    tfm_file: TFMFile,
    scale: Dimen,
}

impl FontMetrics {
    pub fn from_font(font: &Font) -> Option<Self> {
        let font_filename = format!("{}.tfm", font.font_name);
        let font_path = get_path_to_font(&font_filename)?;
        let file = match TFMFile::from_path(&font_path) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("Error loading font: {}", err);
                None
            }
        }?;

        Some(FontMetrics {
            tfm_file: file,
            scale: font.scale,
        })
    }

    #[allow(clippy::disallowed_types)]
    pub fn get_design_size(&self) -> f64 {
        self.tfm_file.get_design_size()
    }

    pub fn get_checksum(&self) -> u32 {
        self.tfm_file.get_checksum()
    }

    pub fn get_width(&self, chr: char) -> Dimen {
        self.tfm_file.get_width(chr, Some(&self.scale))
    }

    pub fn get_height(&self, chr: char) -> Dimen {
        self.tfm_file.get_height(chr, Some(&self.scale))
    }

    pub fn get_depth(&self, chr: char) -> Dimen {
        self.tfm_file.get_depth(chr, Some(&self.scale))
    }

    pub fn get_font_dimension(&self, dimen_number: usize) -> Dimen {
        self.tfm_file
            .get_font_dimension(dimen_number, Some(&self.scale))
    }

    pub fn get_successor(&self, chr: char) -> char {
        self.tfm_file.get_successor(chr)
    }
}

#[cfg(test)]
mod tests {
    use super::FontMetrics;
    use crate::dimension::{Dimen, Unit};
    use crate::font::Font;

    #[test]
    fn it_correctly_loads_font_metrics() {
        let metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        })
        .unwrap();

        assert_eq!(metrics.get_width('a'), Dimen::from_scaled_points(327681));
    }

    #[test]
    fn it_loads_scaled_fonts() {
        let tenpt_metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        })
        .unwrap();

        let fivept_metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(5.0, Unit::Point),
        })
        .unwrap();

        let twentypt_metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(20.0, Unit::Point),
        })
        .unwrap();

        // Not exactly twice due to rounding.
        // \font\tenrm=cmr10 at 10pt \setbox0=\hbox{\tenrm a} \count0=\wd0 \showthe\count0
        assert_eq!(
            tenpt_metrics.get_width('a'),
            Dimen::from_scaled_points(327681)
        );
        // \font\fiverm=cmr10 at 5pt \setbox0=\hbox{\fiverm a} \count0=\wd0 \showthe\count0
        assert_eq!(
            fivept_metrics.get_width('a'),
            Dimen::from_scaled_points(163840)
        );

        // \font\twentyrm=cmr10 at 20pt \setbox0=\hbox{\twentyrm w} \count0=\ht0 \showthe\count0
        assert_eq!(
            twentypt_metrics.get_height('w'),
            Dimen::from_scaled_points(564337)
        );

        // \font\fiverm=cmr10 at 5pt \setbox0=\hbox{\fiverm j} \count0=\dp0 \showthe\count0
        assert_eq!(
            fivept_metrics.get_depth('j'),
            Dimen::from_scaled_points(63715)
        );
    }

    #[test]
    fn it_scales_font_dimensions() {
        let twentypt_metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(20.0, Unit::Point),
        })
        .unwrap();

        assert_eq!(
            twentypt_metrics.get_font_dimension(5),
            Dimen::from_scaled_points(564337)
        );
    }

    #[test]
    fn it_correctly_gets_successors() {
        let cmr_metrics = FontMetrics::from_font(&Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(20.0, Unit::Point),
        })
        .unwrap();
        let cmex_metrics = FontMetrics::from_font(&Font {
            font_name: "cmex10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        })
        .unwrap();

        // i is a vanilla character
        assert_eq!(cmr_metrics.get_successor('i'), 'i');
        // a has a lig-kern program
        assert_eq!(cmr_metrics.get_successor('a'), 'a');
        // \sum has a real successor
        assert_eq!(cmex_metrics.get_successor(0x50 as char), 0x58 as char);
        // \uparrow has an extension recipe
        assert_eq!(cmex_metrics.get_successor(0x78 as char), 0x78 as char);
    }
}

use crate::dimension::{Dimen, Unit};
use crate::font::Font;
use crate::paths::get_path_to_font;
use crate::tfm::TFMFile;

#[derive(Debug)]
pub struct FontMetrics {
    tfm_file: TFMFile,
    design_size: Dimen,
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

        let design_size = file.get_design_size();

        Some(FontMetrics {
            tfm_file: file,
            design_size: Dimen::from_unit(design_size, Unit::Point),
            scale: font.scale.clone(),
        })
    }

    pub fn get_design_size(&self) -> f64 {
        self.tfm_file.get_design_size()
    }

    pub fn get_checksum(&self) -> u32 {
        self.tfm_file.get_checksum()
    }

    fn scale_dimen(&self, dimen: Dimen) -> Dimen {
        Dimen::from_scaled_points(
            (dimen.as_scaled_points() as i64
                * self.scale.as_scaled_points() as i64
                / self.design_size.as_scaled_points() as i64)
                as i32,
        )
    }

    pub fn get_width(&self, chr: char) -> Dimen {
        self.scale_dimen(self.tfm_file.get_width(chr))
    }

    pub fn get_height(&self, chr: char) -> Dimen {
        self.scale_dimen(self.tfm_file.get_height(chr))
    }

    pub fn get_depth(&self, chr: char) -> Dimen {
        self.scale_dimen(self.tfm_file.get_depth(chr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(
            tenpt_metrics.get_width('a') / 2,
            fivept_metrics.get_width('a')
        );

        assert_eq!(
            twentypt_metrics.get_height('w') / 2,
            tenpt_metrics.get_height('w')
        );

        assert_eq!(
            twentypt_metrics.get_depth('j') / 4,
            fivept_metrics.get_depth('j')
        );
    }
}

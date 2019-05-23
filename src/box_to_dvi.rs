use std::collections::HashMap;
use std::io;

use crate::dvi::DVICommand;
use crate::list::HorizontalListElem;
use crate::paths::get_path_to_font;
use crate::tfm::TFMFile;

struct DVIFileWriter {
    commands: Vec<DVICommand>,
    stack_depth: usize,
    curr_font_num: i32,
    font_nums: HashMap<String, i32>,
    next_font_num: i32,
}

fn get_metrics_for_font(font: &str) -> io::Result<TFMFile> {
    let font_file_name = format!("{}.tfm", font);
    let font_path = get_path_to_font(&font_file_name).ok_or(io::Error::new(
        io::ErrorKind::Other,
        format!("Couldn't find file {}", font_file_name),
    ))?;
    TFMFile::from_path(&font_path)
}

impl DVIFileWriter {
    fn new() -> Self {
        DVIFileWriter {
            commands: Vec::new(),
            stack_depth: 0,
            curr_font_num: -1,
            font_nums: HashMap::new(),
            next_font_num: 0,
        }
    }

    fn add_font_def(&mut self, font: &str) -> i32 {
        let font_num = self.next_font_num;
        self.next_font_num = self.next_font_num + 1;

        let metrics = get_metrics_for_font(font)
            .expect(&format!("Error loading font metrics for {}", font));

        self.commands.push(DVICommand::FntDef4 {
            font_num: font_num,
            checksum: metrics.get_checksum(),

            // These are just copied from what TeX produces
            scale: 655360,
            design_size: 655360,

            area: 0,
            length: font.len() as u8,
            font_name: font.to_string(),
        });
        self.font_nums.insert(font.to_string(), font_num);

        font_num
    }

    fn switch_to_font(&mut self, font: &str) {
        let font_num = if let Some(font_num) = self.font_nums.get(font) {
            *font_num
        } else {
            self.add_font_def(font)
        };

        if font_num != self.curr_font_num {
            self.commands.push(DVICommand::Fnt4(font_num));
            self.curr_font_num = font_num;
        }
    }

    fn add_horizontal_list_elem(&mut self, elem: &HorizontalListElem) {
        match elem {
            HorizontalListElem::Char { chr, font } => {
                let command = if (*chr as u8) < 128 {
                    DVICommand::SetCharN(*chr as u8)
                } else {
                    DVICommand::Set1(*chr as u8)
                };

                self.switch_to_font(font);
                self.commands.push(command);
            }

            HorizontalListElem::HSkip(_glue) => {
                panic!("unimplemented");
            }

            HorizontalListElem::Box(_tex_box) => {
                panic!("unimplemented");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_generates_commands_for_chars() {
        let mut writer = DVIFileWriter::new();
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr10".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 200 as char,
            font: "cmr10".to_string(),
        });

        // The commands start with a fnt_def4 and fnt4 command, then come the
        // chars.
        assert_eq!(writer.commands.len(), 4);

        assert_eq!(
            &writer.commands[2..],
            &[DVICommand::SetCharN(97), DVICommand::Set1(200)]
        );
    }

    #[test]
    fn it_generates_fnt_commands() {
        let mut writer = DVIFileWriter::new();
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr10".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr10".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr7".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr7".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr10".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmtt10".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr7".to_string(),
        });
        writer.add_horizontal_list_elem(&HorizontalListElem::Char {
            chr: 'a',
            font: "cmr10".to_string(),
        });

        let cmr10_metrics = get_metrics_for_font("cmr10").unwrap();
        let cmr7_metrics = get_metrics_for_font("cmr7").unwrap();
        let cmtt10_metrics = get_metrics_for_font("cmtt10").unwrap();

        assert_eq!(
            writer.commands,
            vec![
                DVICommand::FntDef4 {
                    font_num: 0,
                    checksum: cmr10_metrics.get_checksum(),
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 5,
                    font_name: "cmr10".to_string(),
                },
                DVICommand::Fnt4(0),
                DVICommand::SetCharN(97),
                DVICommand::SetCharN(97),
                DVICommand::FntDef4 {
                    font_num: 1,
                    checksum: cmr7_metrics.get_checksum(),
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 4,
                    font_name: "cmr7".to_string(),
                },
                DVICommand::Fnt4(1),
                DVICommand::SetCharN(97),
                DVICommand::SetCharN(97),
                DVICommand::Fnt4(0),
                DVICommand::SetCharN(97),
                DVICommand::FntDef4 {
                    font_num: 2,
                    checksum: cmtt10_metrics.get_checksum(),
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 6,
                    font_name: "cmtt10".to_string(),
                },
                DVICommand::Fnt4(2),
                DVICommand::SetCharN(97),
                DVICommand::Fnt4(1),
                DVICommand::SetCharN(97),
                DVICommand::Fnt4(0),
                DVICommand::SetCharN(97),
            ]
        );
    }
}

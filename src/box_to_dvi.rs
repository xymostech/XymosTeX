use std::collections::HashMap;

use crate::dvi::DVICommand;
use crate::list::HorizontalListElem;

struct DVIFileWriter {
    commands: Vec<DVICommand>,
    stack_depth: usize,
    curr_font_num: usize,
    font_nums: HashMap<String, usize>,
}

impl DVIFileWriter {
    fn new() -> Self {
        DVIFileWriter {
            commands: Vec::new(),
            stack_depth: 0,
            curr_font_num: 0,
            font_nums: HashMap::new(),
        }
    }

    fn add_horizontal_list_elem(&mut self, elem: &HorizontalListElem) {
        match elem {
            HorizontalListElem::Char { chr, font: _ } => {
                let command = if (*chr as u8) < 128 {
                    DVICommand::SetCharN(*chr as u8)
                } else {
                    DVICommand::Set1(*chr as u8)
                };

                // TODO(xymostech): Optionally add a fnt_def command and/or
                // a fnt_num command to switch to the appropriate font
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

        assert_eq!(
            writer.commands,
            vec![DVICommand::SetCharN(97), DVICommand::Set1(200),]
        );
    }
}

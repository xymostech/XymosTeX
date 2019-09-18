use std::collections::HashMap;
use std::io;

use crate::boxes::GlueSetRatio;
use crate::boxes::{HorizontalBox, TeXBox};
use crate::dvi::DVICommand;
use crate::list::{HorizontalListElem, VerticalListElem};
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

    fn add_box(&mut self, tex_box: &TeXBox) {
        self.commands.push(DVICommand::Push);

        match tex_box {
            TeXBox::HorizontalBox(hbox) => {
                for elem in &hbox.list {
                    self.add_horizontal_list_elem(&elem, &hbox.glue_set_ratio);
                }
            }
            TeXBox::VerticalBox(_) => {
                panic!("unimplemented");
            }
        }

        self.commands.push(DVICommand::Pop);
    }

    fn add_vertical_list_elem(
        &mut self,
        elem: &VerticalListElem,
        glue_set_ratio: &Option<GlueSetRatio>,
    ) {
        match elem {
            VerticalListElem::VSkip(glue) => {
                let move_amount = if let Some(set_ratio) = glue_set_ratio {
                    set_ratio.apply_to_glue(glue)
                } else {
                    glue.space
                };

                self.commands
                    .push(DVICommand::Down4(move_amount.as_scaled_points()));
            }

            VerticalListElem::Box(_) => {
                panic!("unimplemented");
            }
        }
    }

    fn add_horizontal_list_elem(
        &mut self,
        elem: &HorizontalListElem,
        glue_set_ratio: &Option<GlueSetRatio>,
    ) {
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

            HorizontalListElem::HSkip(glue) => {
                let move_amount = if let Some(set_ratio) = glue_set_ratio {
                    set_ratio.apply_to_glue(glue)
                } else {
                    glue.space
                };

                self.commands
                    .push(DVICommand::Right4(move_amount.as_scaled_points()));
            }

            HorizontalListElem::Box(tex_box) => {
                self.add_box(tex_box);
                self.commands.push(DVICommand::Right4(
                    tex_box.width().as_scaled_points(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::GlueSetRatioKind;
    use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
    use crate::glue::Glue;

    #[test]
    fn it_generates_commands_for_chars() {
        let mut writer = DVIFileWriter::new();
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 200 as char,
                font: "cmr10".to_string(),
            },
            &None,
        );

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
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr7".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr7".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmtt10".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr7".to_string(),
            },
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            },
            &None,
        );

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

    #[test]
    fn it_adds_hskips() {
        let mut writer = DVIFileWriter::new();

        // No stretch/shrink
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue::from_dimen(Dimen::from_unit(
                2.0,
                Unit::Point,
            ))),
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue::from_dimen(Dimen::from_unit(
                2.0,
                Unit::Point,
            ))),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 2.0)),
        );

        // Finite stretch
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 2.0)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -1.5)),
        );

        // Finite shrink
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );

        // Infinite stretch
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fill, 1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -0.5)),
        );

        // Infinite shrink
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &None,
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fill, -1.5)),
        );
        writer.add_horizontal_list_elem(
            &HorizontalListElem::HSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 1.5)),
        );

        assert_eq!(
            &writer.commands,
            &[
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536 + 3 * 3 * 65536 / 2),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(4 * 65536),
                DVICommand::Right4(4 * 65536 - 2 * 65536 / 2),
                DVICommand::Right4(4 * 65536),
                DVICommand::Right4(4 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536 + 3 * 3 * 65536 / 2),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(2 * 65536),
                DVICommand::Right4(6 * 65536),
                DVICommand::Right4(6 * 65536 - 3 * 65536),
                DVICommand::Right4(6 * 65536),
                DVICommand::Right4(6 * 65536),
                DVICommand::Right4(6 * 65536),
            ]
        );
    }

    #[derive(Debug)]
    enum MaybeEquals<T> {
        Equals(T),
        Anything,
    }

    fn assert_matches<T: std::fmt::Debug + std::cmp::PartialEq>(
        reals: &[T],
        matches: &[MaybeEquals<T>],
    ) {
        if reals.len() != matches.len() {
            panic!("{:?} doesn't have the same length as {:?}", reals, matches);
        }

        for (i, (real, matcher)) in reals.iter().zip(matches.iter()).enumerate()
        {
            match matcher {
                MaybeEquals::Equals(m) => {
                    if real != m {
                        panic!("{:?} doesn't match {:?}: element {} is different: {:?} vs {:?}", reals, matches, i, real, m);
                    }
                }
                MaybeEquals::Anything => (),
            }
        }
    }

    #[test]
    fn it_adds_basic_boxes() {
        let mut writer = DVIFileWriter::new();

        let metrics = get_metrics_for_font("cmr10").unwrap();

        let box1 = TeXBox::HorizontalBox(HorizontalBox {
            height: metrics.get_width('a'),
            depth: metrics.get_depth('a'),
            width: metrics.get_width('a'),

            list: vec![HorizontalListElem::Char {
                chr: 'a',
                font: "cmr10".to_string(),
            }],
            glue_set_ratio: None,
        });

        writer.add_box(&box1.clone());
        writer.add_horizontal_list_elem(
            &HorizontalListElem::Box(box1.clone()),
            &None,
        );

        assert_matches(
            &writer.commands,
            &[
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN(97)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Equals(DVICommand::SetCharN(97)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Right4(
                    metrics.get_width('a').as_scaled_points(),
                )),
            ],
        );
    }

    #[test]
    fn it_adds_vskips() {
        let mut writer = DVIFileWriter::new();

        // No stretch/shrink
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue::from_dimen(Dimen::from_unit(
                2.0,
                Unit::Point,
            ))),
            &None,
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue::from_dimen(Dimen::from_unit(
                2.0,
                Unit::Point,
            ))),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 2.0)),
        );

        // Finite stretch
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &None,
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 2.0)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::from_unit(3.0, Unit::Point)),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -1.5)),
        );

        // Finite shrink
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &None,
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(4.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );

        // Infinite stretch
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &None,
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, 1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fill, 1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(2.0, Unit::Point),
                stretch: SpringDimen::FilDimen(FilDimen::new(
                    FilKind::Fil,
                    3.0,
                )),
                shrink: SpringDimen::Dimen(Dimen::zero()),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -0.5)),
        );

        // Infinite shrink
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &None,
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, -1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Finite, -0.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fill, -1.5)),
        );
        writer.add_vertical_list_elem(
            &VerticalListElem::VSkip(Glue {
                space: Dimen::from_unit(6.0, Unit::Point),
                stretch: SpringDimen::Dimen(Dimen::zero()),
                shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 2.0)),
            }),
            &Some(GlueSetRatio::from(GlueSetRatioKind::Fil, 1.5)),
        );

        assert_eq!(
            &writer.commands,
            &[
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536 + 3 * 3 * 65536 / 2),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(4 * 65536),
                DVICommand::Down4(4 * 65536 - 2 * 65536 / 2),
                DVICommand::Down4(4 * 65536),
                DVICommand::Down4(4 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536 + 3 * 3 * 65536 / 2),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(2 * 65536),
                DVICommand::Down4(6 * 65536),
                DVICommand::Down4(6 * 65536 - 3 * 65536),
                DVICommand::Down4(6 * 65536),
                DVICommand::Down4(6 * 65536),
                DVICommand::Down4(6 * 65536),
            ]
        );
    }
}

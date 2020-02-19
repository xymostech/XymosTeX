use std::collections::HashMap;
use std::io;

use crate::boxes::GlueSetRatio;
use crate::boxes::TeXBox;
use crate::dvi::{DVICommand, DVIFile};
use crate::list::{HorizontalListElem, VerticalListElem};
use crate::paths::get_path_to_font;
use crate::tfm::TFMFile;

struct DVIFileWriter {
    commands: Vec<DVICommand>,
    last_page_start: i32,
    curr_font_num: i32,
    font_nums: HashMap<String, i32>,
    next_font_num: i32,
    num: u32,
    den: u32,
    mag: u32,
    num_pages: u16,
    max_stack_depth: u16,
    curr_stack_depth: u16,
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
            last_page_start: -1,
            curr_font_num: -1,
            font_nums: HashMap::new(),
            next_font_num: 0,
            num: 0,
            den: 0,
            mag: 0,
            num_pages: 0,
            max_stack_depth: 0,
            curr_stack_depth: 0,
        }
    }

    fn add_font_def_with_metrics(
        &mut self,
        font: &str,
        metrics: &TFMFile,
        font_num: i32,
    ) {
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
    }

    fn add_font_def(&mut self, font: &str) -> i32 {
        let font_num = self.next_font_num;
        self.next_font_num = self.next_font_num + 1;

        let metrics = get_metrics_for_font(font)
            .expect(&format!("Error loading font metrics for {}", font));

        self.add_font_def_with_metrics(font, &metrics, font_num);
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
        self.curr_stack_depth += 1;
        if self.curr_stack_depth > self.max_stack_depth {
            self.max_stack_depth = self.curr_stack_depth;
        }

        match tex_box {
            TeXBox::HorizontalBox(hbox) => {
                for elem in &hbox.list {
                    self.add_horizontal_list_elem(&elem, &hbox.glue_set_ratio);
                }
            }
            TeXBox::VerticalBox(vbox) => {
                for elem in &vbox.list {
                    self.add_vertical_list_elem(&elem, &vbox.glue_set_ratio);
                }
            }
        }

        self.commands.push(DVICommand::Pop);
        self.curr_stack_depth -= 1;
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

            VerticalListElem::Box(tex_box) => {
                self.add_box(tex_box);
                self.commands.push(DVICommand::Down4(
                    tex_box.height().as_scaled_points()
                        + tex_box.depth().as_scaled_points(),
                ));
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

    fn total_byte_size(&self) -> usize {
        self.commands
            .iter()
            .map(|command| command.byte_size())
            .sum::<usize>()
    }

    fn add_page(
        &mut self,
        elems: &[VerticalListElem],
        glue_set_ratio: &Option<GlueSetRatio>,
        cs: [i32; 10],
    ) {
        self.num_pages += 1;

        let old_last_page_start = self.last_page_start;
        self.last_page_start = self.total_byte_size() as i32;
        self.commands.push(DVICommand::Bop {
            cs: cs,
            pointer: old_last_page_start,
        });

        self.curr_font_num = -1;
        for elem in elems {
            self.add_vertical_list_elem(elem, glue_set_ratio);
        }

        self.commands.push(DVICommand::Eop);
    }

    fn start(&mut self, unit_frac: (u32, u32), mag: u32, comment: Vec<u8>) {
        let (num, den) = unit_frac;

        self.num = num;
        self.den = den;
        self.mag = mag;

        self.commands.push(DVICommand::Pre {
            format: 2,
            num: num,
            den: den,
            mag: mag,
            comment: comment,
        });
    }

    fn end(&mut self) {
        let post_pointer = self.total_byte_size();

        self.commands.push(DVICommand::Post {
            pointer: self.last_page_start as u32,
            num: self.num,
            den: self.den,
            mag: self.mag,
            max_page_height: 43725786, // TODO(fixme)
            max_page_width: 30785863,  // TODO(fixme)
            max_stack_depth: self.max_stack_depth,
            num_pages: self.num_pages,
        });

        for (font, font_num) in std::mem::take(&mut self.font_nums) {
            let metrics = get_metrics_for_font(&font)
                .expect(&format!("Error loading font metrics for {}", font));

            self.add_font_def_with_metrics(&font, &metrics, font_num);
        }

        let total_size = self.total_byte_size();

        self.commands.push(DVICommand::PostPost {
            post_pointer: post_pointer as u32,
            format: 2,
            tail: 4 + ((total_size + 6) % 4) as u8,
        });
    }

    fn to_file(&self) -> DVIFile {
        DVIFile {
            commands: self.commands.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::{GlueSetRatioKind, HorizontalBox, VerticalBox};
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
            panic!(
                "{:?} doesn't have the same length as {:?} ({:?} vs {:?})",
                reals,
                matches,
                reals.len(),
                matches.len()
            );
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
    fn it_adds_basic_horizontal_boxes() {
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

    #[test]
    fn it_adds_basic_vertical_boxes() {
        let mut writer = DVIFileWriter::new();

        let metrics = get_metrics_for_font("cmr10").unwrap();

        let hbox = TeXBox::HorizontalBox(HorizontalBox {
            height: metrics.get_height('g'),
            depth: metrics.get_depth('g'),
            width: metrics.get_width('g'),

            list: vec![HorizontalListElem::Char {
                chr: 'g',
                font: "cmr10".to_string(),
            }],
            glue_set_ratio: None,
        });

        let vbox = TeXBox::VerticalBox(VerticalBox {
            height: *hbox.height(),
            depth: *hbox.depth() + Dimen::from_unit(2.0, Unit::Point),
            width: *hbox.width(),

            list: vec![
                VerticalListElem::Box(hbox.clone()),
                VerticalListElem::VSkip(Glue {
                    space: Dimen::from_unit(2.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }),
            ],
            glue_set_ratio: None,
        });

        writer.add_box(&vbox.clone());
        writer.add_vertical_list_elem(
            &VerticalListElem::Box(vbox.clone()),
            &None,
        );

        assert_matches(
            &writer.commands,
            &[
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN(103)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    hbox.height().as_scaled_points()
                        + hbox.depth().as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Down4(131072)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Equals(DVICommand::SetCharN(103)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    hbox.height().as_scaled_points()
                        + hbox.depth().as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Down4(131072)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    hbox.height().as_scaled_points()
                        + hbox.depth().as_scaled_points()
                        + 131072,
                )),
            ],
        );
    }

    use crate::testing::with_parser;

    #[test]
    fn it_adds_pages() {
        let mut writer = DVIFileWriter::new();

        let metrics = get_metrics_for_font("cmr10").unwrap();

        with_parser(
            &[
                r"\vbox{\noindent g\vskip 0pt\noindent a}%",
                r"\vbox{\noindent q}%",
                r"\vbox{\noindent a}%",
            ],
            |parser| {
                let page1 = parser.parse_box().unwrap();
                let page2 = parser.parse_box().unwrap();
                let page3 = parser.parse_box().unwrap();

                if let TeXBox::VerticalBox(vbox1) = page1 {
                    writer.add_page(
                        &vbox1.list,
                        &vbox1.glue_set_ratio,
                        [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    );
                } else {
                    panic!("page1 wasn't a vertical box: {:?}", page1);
                }

                if let TeXBox::VerticalBox(vbox2) = page2 {
                    writer.add_page(
                        &vbox2.list,
                        &vbox2.glue_set_ratio,
                        [2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    );
                } else {
                    panic!("page2 wasn't a vertical box: {:?}", page2);
                }

                if let TeXBox::VerticalBox(vbox3) = page3 {
                    writer.add_page(
                        &vbox3.list,
                        &vbox3.glue_set_ratio,
                        [3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    );
                } else {
                    panic!("page3 wasn't a vertical box: {:?}", page3);
                }
            },
        );

        assert_matches(
            &writer.commands,
            &[
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                }),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN('g' as u8)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    metrics.get_height('g').as_scaled_points()
                        + metrics.get_depth('g').as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Down4(0)),
                MaybeEquals::Equals(DVICommand::Down4(376833)), // FIXME
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Equals(DVICommand::SetCharN('a' as u8)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    metrics.get_height('a').as_scaled_points()
                        + metrics.get_depth('a').as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: 0,
                }),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN('q' as u8)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    metrics.get_height('q').as_scaled_points()
                        + metrics.get_depth('q').as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: 101,
                }),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN('a' as u8)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    metrics.get_height('a').as_scaled_points()
                        + metrics.get_depth('a').as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Eop),
            ],
        );
    }

    #[test]
    fn it_adds_basic_pre_and_post() {
        let mut writer = DVIFileWriter::new();

        let metrics = get_metrics_for_font("cmr10").unwrap();

        writer.start(
            (25400000, 473628672),
            1000,
            "hello, world!".as_bytes().to_vec(),
        );

        with_parser(&[r"\vbox{\noindent a}%"], |parser| {
            let page1 = parser.parse_box().unwrap();
            if let TeXBox::VerticalBox(vbox1) = page1 {
                writer.add_page(
                    &vbox1.list,
                    &vbox1.glue_set_ratio,
                    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                );
            } else {
                panic!("page isn't a vertical box: {:?}", page1);
            }
        });

        writer.end();

        assert_matches(
            &writer.commands,
            &[
                MaybeEquals::Equals(DVICommand::Pre {
                    format: 2,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    comment: vec![
                        'h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8,
                        ',' as u8, ' ' as u8, 'w' as u8, 'o' as u8, 'r' as u8,
                        'l' as u8, 'd' as u8, '!' as u8,
                    ],
                }),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                }),
                MaybeEquals::Equals(DVICommand::Push),
                MaybeEquals::Anything,
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::SetCharN('a' as u8)),
                MaybeEquals::Equals(DVICommand::Pop),
                MaybeEquals::Equals(DVICommand::Down4(
                    metrics.get_height('a').as_scaled_points()
                        + metrics.get_depth('a').as_scaled_points(),
                )),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Post {
                    pointer: 28,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    max_page_height: 43725786,
                    max_page_width: 30785863,
                    max_stack_depth: 1,
                    num_pages: 1,
                }),
                MaybeEquals::Anything,
                MaybeEquals::Equals(DVICommand::PostPost {
                    post_pointer: 111,
                    format: 2,
                    tail: 6,
                }),
            ],
        );

        // The total size of the file should be a multiple of 4
        assert_eq!(writer.total_byte_size() % 4, 0);

        let first_font_def = &writer.commands[3];
        let last_font_def = &writer.commands[10];

        // The font defs in the post should match the defs in the pages
        assert_eq!(first_font_def, last_font_def);
    }

    #[test]
    fn it_calculates_num_pages_correctly() {
        let mut writer = DVIFileWriter::new();

        let metrics = get_metrics_for_font("cmr10").unwrap();

        writer.start((25400000, 473628672), 1000, vec![]);

        writer.add_page(&[], &None, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        writer.add_page(&[], &None, [2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        writer.add_page(&[], &None, [3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        writer.end();

        assert_matches(
            &writer.commands,
            &[
                MaybeEquals::Equals(DVICommand::Pre {
                    format: 2,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    comment: vec![],
                }),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                }),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: 15,
                }),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: 61,
                }),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Post {
                    pointer: 107,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    max_page_height: 43725786,
                    max_page_width: 30785863,
                    max_stack_depth: 0,
                    num_pages: 3,
                }),
                MaybeEquals::Equals(DVICommand::PostPost {
                    post_pointer: 153,
                    format: 2,
                    tail: 4,
                }),
            ],
        );
    }

    #[test]
    fn it_converts_to_a_file() {
        let mut writer = DVIFileWriter::new();

        writer.start((25400000, 473628672), 1000, vec![]);

        writer.add_page(&[], &None, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        writer.end();

        let file = writer.to_file();

        assert_matches(
            &file.commands,
            &[
                MaybeEquals::Equals(DVICommand::Pre {
                    format: 2,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    comment: vec![],
                }),
                MaybeEquals::Equals(DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                }),
                MaybeEquals::Equals(DVICommand::Eop),
                MaybeEquals::Equals(DVICommand::Post {
                    pointer: 15,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    max_page_height: 43725786,
                    max_page_width: 30785863,
                    max_stack_depth: 0,
                    num_pages: 1,
                }),
                MaybeEquals::Equals(DVICommand::PostPost {
                    post_pointer: 61,
                    format: 2,
                    tail: 4,
                }),
            ],
        );
    }
}

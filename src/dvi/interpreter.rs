use std::collections::{HashMap, HashSet};
use std::iter::Peekable;

use super::file::{DVICommand, DVIFile};
use crate::paths::get_path_to_font;
use crate::tfm::TFMFile;

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum DVIOutputElement {
    Character { char: i32, font: String },
}

pub type DVIPageOutput = HashMap<(i32, i32), HashSet<DVIOutputElement>>;

#[derive(Clone)]
struct DVIStateStack {
    // current position on the page, (h, v)
    h: i32,
    v: i32,

    // spacing amounts
    w: i32,
    x: i32,
    y: i32,
    z: i32,
}

struct DVIState {
    // map of font number to font data
    fonts: HashMap<i32, (TFMFile, String)>,

    // current font num
    f: Option<i32>,

    // The stack of state values
    stack: Vec<DVIStateStack>,
}

impl DVIState {
    fn add_font(&mut self, font_num: i32, font_name: &str) {
        let font_filename = format!("{}.tfm", font_name);
        let font_path =
            get_path_to_font(&font_filename).expect("Couldn't find font");
        let metrics =
            TFMFile::from_path(&font_path).expect("Failed to read font");

        self.fonts
            .insert(font_num, (metrics, font_name.to_string()));
    }

    fn current_font(&self) -> &(TFMFile, String) {
        let font_num = self.f.expect("No font selected");
        &self.fonts[&font_num]
    }

    fn curr_stack(&mut self) -> &mut DVIStateStack {
        let len = self.stack.len();
        &mut self.stack[len - 1]
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.stack.push(DVIStateStack {
            h: 0,
            v: 0,
            w: 0,
            x: 0,
            y: 0,
            z: 0,
        });
    }

    fn push_stack(&mut self) {
        let new_elem = self.curr_stack().clone();
        self.stack.push(new_elem)
    }

    fn pop_stack(&mut self) {
        if self.stack.len() == 1 {
            panic!("Cannot pop without corresponding push");
        }

        self.stack.pop();
    }
}

fn add_to_page(
    page: &mut DVIPageOutput,
    pos: (i32, i32),
    element: DVIOutputElement,
) {
    match page.get_mut(&pos) {
        Some(elems) => {
            elems.insert(element);
        }
        None => {
            let mut set = HashSet::new();
            set.insert(element);
            page.insert(pos, set);
        }
    }
}

fn interpret_page<'a, I>(
    state: &mut DVIState,
    commands: &mut Peekable<I>,
) -> DVIPageOutput
where
    I: Iterator<Item = &'a DVICommand>,
{
    let mut page = HashMap::new();

    match commands.next().expect("Missing Bop") {
        DVICommand::Bop { .. } => {}
        other => panic!("Expecting Bop, got {:?}", other),
    }

    state.f = None;
    state.reset_stack();

    loop {
        match commands.next().unwrap() {
            DVICommand::Eop => break,
            DVICommand::Push => state.push_stack(),
            DVICommand::Pop => state.pop_stack(),
            DVICommand::Right3(b) => {
                state.curr_stack().h += b;
            }
            DVICommand::Right4(b) => {
                state.curr_stack().h += b;
            }
            DVICommand::W0 => {
                state.curr_stack().h += state.curr_stack().w;
            }
            DVICommand::W3(b) => {
                state.curr_stack().w = *b;
                state.curr_stack().h += b;
            }
            DVICommand::Down3(a) => {
                state.curr_stack().v += a;
            }
            DVICommand::Down4(a) => {
                state.curr_stack().v += a;
            }
            DVICommand::Y0 => {
                state.curr_stack().v += state.curr_stack().y;
            }
            DVICommand::Y3(a) => {
                state.curr_stack().y = *a;
                state.curr_stack().v += a;
            }
            DVICommand::FntDef1 {
                font_num,
                font_name,
                ..
            } => {
                state.add_font(*font_num as i32, font_name);
            }
            DVICommand::FntDef4 {
                font_num,
                font_name,
                ..
            } => {
                state.add_font(*font_num, font_name);
            }
            DVICommand::FntNumN(f) => {
                state.f = Some(*f as i32);
            }
            DVICommand::Fnt4(f) => {
                state.f = Some(*f);
            }
            DVICommand::SetCharN(n) => {
                {
                    let font_name = {
                        let (_, font_name) = state.current_font();
                        font_name.to_string()
                    };
                    let stack = state.curr_stack();

                    add_to_page(
                        &mut page,
                        (stack.h, stack.v),
                        DVIOutputElement::Character {
                            char: *n as i32,
                            font: font_name,
                        },
                    );
                }

                let shift_width = {
                    let (metrics, _) = state.current_font();
                    metrics.get_width(*n as char)
                };
                state.curr_stack().h += shift_width.as_scaled_points();
            }
            other => panic!("unknown command: {:?}", other),
        }
    }

    page
}

/// This interprets the commands of DVI file into the placements of characters
/// and other DVI elements on the various pages. This does very minor
/// validation that the structure of the DVI file is correct, and panics if
/// anything is wrong.
pub fn interpret_dvi_file(file: DVIFile) -> Vec<DVIPageOutput> {
    let mut commands = file.commands.iter().peekable();

    match commands.next().expect("Missing Pre") {
        DVICommand::Pre {
            format,
            num,
            den,
            mag,
            ..
        } => {
            if *format != 2 {
                panic!("Unknown format: {}", format);
            }

            if *num != 25400000 || *den != 473628672 {
                panic!("Only handling default num/den");
            }

            if *mag != 1000 {
                panic!("Only handling default mag (1000)");
            }
        }

        _ => panic!("First command must be pre!"),
    };

    let mut pages = Vec::new();
    let mut state = DVIState {
        fonts: HashMap::new(),
        f: None,
        stack: vec![DVIStateStack {
            h: 0,
            v: 0,
            w: 0,
            x: 0,
            y: 0,
            z: 0,
        }],
    };

    loop {
        if let DVICommand::Post { .. } = commands.peek().unwrap() {
            break;
        }

        let page = interpret_page(&mut state, &mut commands);
        pages.push(page);
    }

    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_metrics_for_font(font: &str) -> TFMFile {
        let font_file_name = format!("{}.tfm", font);
        let font_path = get_path_to_font(&font_file_name).unwrap();
        TFMFile::from_path(&font_path).unwrap()
    }

    fn empty_state() -> DVIState {
        DVIState {
            fonts: HashMap::new(),
            f: None,
            stack: vec![DVIStateStack {
                h: 0,
                v: 0,
                w: 0,
                x: 0,
                y: 0,
                z: 0,
            }],
        }
    }

    fn interpret_page_from_commands(file: Vec<DVICommand>) -> DVIPageOutput {
        let mut state = empty_state();
        let mut commands = file.iter().peekable();

        interpret_page(&mut state, &mut commands)
    }

    macro_rules! set {
        ( $( $x:expr ),* ) => {
            {
                let mut temp_set = HashSet::new();
                $(
                    temp_set.insert($x);
                )*
                temp_set
            }
        };
    }

    #[test]
    fn test_interpreting_whole_file() {
        let pages = interpret_dvi_file(DVIFile {
            commands: vec![
                DVICommand::Pre {
                    format: 2,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    comment: vec![],
                },
                DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: -1,
                },
                DVICommand::FntDef1 {
                    font_num: 0,
                    checksum: 305419896,
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 5,
                    font_name: "cmr10".to_string(),
                },
                DVICommand::FntNumN(0),
                DVICommand::SetCharN(63),
                DVICommand::Eop,
                DVICommand::Bop {
                    cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    pointer: 18,
                },
                DVICommand::FntNumN(0),
                DVICommand::SetCharN(89),
                DVICommand::Eop,
                DVICommand::Post {
                    pointer: 18,
                    num: 25400000,
                    den: 473628672,
                    mag: 1000,
                    max_page_width: 65536,
                    max_page_height: 65536,
                    max_stack_depth: 1,
                    num_pages: 1,
                },
                DVICommand::FntDef1 {
                    font_num: 0,
                    checksum: 305419896,
                    scale: 655360,
                    design_size: 655360,
                    area: 0,
                    length: 5,
                    font_name: "cmr10".to_string(),
                },
                DVICommand::PostPost {
                    post_pointer: 128,
                    format: 2,
                    tail: 6,
                },
            ],
        });

        assert_eq!(pages.len(), 2);

        assert_eq!(pages[0].len(), 1);
        assert_eq!(
            pages[0].get(&(0, 0)),
            Some(&set![DVIOutputElement::Character {
                char: 63,
                font: "cmr10".to_string(),
            }])
        );

        assert_eq!(pages[1].len(), 1);
        assert_eq!(
            pages[1].get(&(0, 0)),
            Some(&set![DVIOutputElement::Character {
                char: 89,
                font: "cmr10".to_string(),
            }])
        );
    }

    #[test]
    fn it_interprets_characters() {
        let page = interpret_page_from_commands(vec![
            DVICommand::Bop {
                cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                pointer: -1,
            },
            DVICommand::FntDef1 {
                font_num: 0,
                checksum: 305419896,
                scale: 655360,
                design_size: 655360,
                area: 0,
                length: 5,
                font_name: "cmr10".to_string(),
            },
            DVICommand::FntNumN(0),
            DVICommand::SetCharN(123),
            DVICommand::Eop,
        ]);

        assert_eq!(page.len(), 1);
        assert_eq!(
            page.get(&(0, 0)),
            Some(&set![DVIOutputElement::Character {
                char: 123,
                font: "cmr10".to_string(),
            }])
        );
    }

    #[test]
    fn it_adds_char_width_when_setting_chars() {
        let page = interpret_page_from_commands(vec![
            DVICommand::Bop {
                cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                pointer: -1,
            },
            DVICommand::FntDef1 {
                font_num: 0,
                checksum: 305419896,
                scale: 655360,
                design_size: 655360,
                area: 0,
                length: 5,
                font_name: "cmr10".to_string(),
            },
            DVICommand::FntNumN(0),
            DVICommand::SetCharN(67),
            DVICommand::SetCharN(67),
            DVICommand::Eop,
        ]);

        let metrics = get_metrics_for_font("cmr10");

        assert_eq!(page.len(), 2);
        assert_eq!(
            page.get(&(0, 0)),
            Some(&set![DVIOutputElement::Character {
                char: 67,
                font: "cmr10".to_string(),
            }])
        );
        assert_eq!(
            page.get(&(metrics.get_width('C').as_scaled_points(), 0)),
            Some(&set![DVIOutputElement::Character {
                char: 67,
                font: "cmr10".to_string(),
            }])
        );
    }

    #[test]
    fn it_handles_movement_commands() {
        let page = interpret_page_from_commands(vec![
            DVICommand::Bop {
                cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                pointer: -1,
            },
            DVICommand::FntDef1 {
                font_num: 0,
                checksum: 305419896,
                scale: 655360,
                design_size: 655360,
                area: 0,
                length: 5,
                font_name: "cmr10".to_string(),
            },
            DVICommand::FntNumN(0),
            DVICommand::Push,
            DVICommand::Right3(1000),
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Push,
            DVICommand::Right4(2000),
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Push,
            DVICommand::W3(1500),
            DVICommand::W0,
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Push,
            DVICommand::Down3(1000),
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Push,
            DVICommand::Down4(2000),
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Push,
            DVICommand::Y3(1500),
            DVICommand::Y0,
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::Eop,
        ]);

        let char_vec = set![DVIOutputElement::Character {
            char: 67,
            font: "cmr10".to_string(),
        }];

        assert_eq!(page.get(&(1000, 0)), Some(&char_vec));
        assert_eq!(page.get(&(2000, 0)), Some(&char_vec));
        assert_eq!(page.get(&(3000, 0)), Some(&char_vec));
        assert_eq!(page.get(&(0, 1000)), Some(&char_vec));
        assert_eq!(page.get(&(0, 2000)), Some(&char_vec));
        assert_eq!(page.get(&(0, 3000)), Some(&char_vec));
    }

    #[test]
    fn it_handles_multiple_characters_in_the_same_location() {
        let page = interpret_page_from_commands(vec![
            DVICommand::Bop {
                cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                pointer: -1,
            },
            DVICommand::FntDef1 {
                font_num: 0,
                checksum: 305419896,
                scale: 655360,
                design_size: 655360,
                area: 0,
                length: 5,
                font_name: "cmr10".to_string(),
            },
            DVICommand::FntNumN(0),
            DVICommand::Push,
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::SetCharN(68),
            DVICommand::Eop,
        ]);

        assert_eq!(page.len(), 1);
        assert_eq!(
            page.get(&(0, 0)),
            Some(&set![
                DVIOutputElement::Character {
                    char: 67,
                    font: "cmr10".to_string(),
                },
                DVIOutputElement::Character {
                    char: 68,
                    font: "cmr10".to_string(),
                }
            ])
        );
    }

    #[test]
    fn it_handles_pushing_and_popping() {
        let page = interpret_page_from_commands(vec![
            DVICommand::Bop {
                cs: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                pointer: -1,
            },
            DVICommand::FntDef1 {
                font_num: 0,
                checksum: 305419896,
                scale: 655360,
                design_size: 655360,
                area: 0,
                length: 5,
                font_name: "cmr10".to_string(),
            },
            DVICommand::FntNumN(0),
            DVICommand::Push,
            DVICommand::Push,
            DVICommand::W3(1000),
            DVICommand::Y3(1000),
            DVICommand::SetCharN(67),
            DVICommand::Pop,
            DVICommand::W0,
            DVICommand::Y0,
            DVICommand::SetCharN(68),
            DVICommand::Pop,
            DVICommand::Down4(2000),
            DVICommand::Push,
            DVICommand::SetCharN(69),
            DVICommand::Push,
            DVICommand::SetCharN(70),
            DVICommand::Pop,
            DVICommand::SetCharN(71),
            DVICommand::Pop,
            DVICommand::Eop,
        ]);

        let metrics = get_metrics_for_font("cmr10");

        assert_eq!(page.len(), 4);
        assert_eq!(
            page.get(&(0, 0)),
            Some(&set![DVIOutputElement::Character {
                char: 68,
                font: "cmr10".to_string(),
            }])
        );
        assert_eq!(
            page.get(&(1000, 1000)),
            Some(&set![DVIOutputElement::Character {
                char: 67,
                font: "cmr10".to_string(),
            }])
        );
        assert_eq!(
            page.get(&(0, 2000)),
            Some(&set![DVIOutputElement::Character {
                char: 69,
                font: "cmr10".to_string(),
            }])
        );
        assert_eq!(
            page.get(&(metrics.get_width('E').as_scaled_points(), 2000)),
            Some(&set![
                DVIOutputElement::Character {
                    char: 70,
                    font: "cmr10".to_string(),
                },
                DVIOutputElement::Character {
                    char: 71,
                    font: "cmr10".to_string(),
                }
            ])
        );
    }
}

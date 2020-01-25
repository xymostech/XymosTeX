#[allow(dead_code)] // TODO(xymostech): remove this once all of these are used
#[derive(Debug, PartialEq)]
pub enum DVICommand {
    SetCharN(u8),
    Set1(u8),
    Set2(u16),
    Set3(u32),
    Set4(i32),
    SetRule {
        height: i32,
        width: i32,
    },
    Put1(u8),
    Put2(u16),
    Put3(u32),
    Put4(i32),
    PutRule {
        height: i32,
        width: i32,
    },
    Nop,
    Bop {
        cs: [i32; 10],
        pointer: i32,
    },
    Eop,
    Push,
    Pop,
    Right1(i8),
    Right2(i16),
    Right3(i32),
    Right4(i32),
    W0,
    W1(i8),
    W2(i16),
    W3(i32),
    W4(i32),
    X0,
    X1(i8),
    X2(i16),
    X3(i32),
    X4(i32),
    Down1(i8),
    Down2(i16),
    Down3(i32),
    Down4(i32),
    Y0,
    Y1(i8),
    Y2(i16),
    Y3(i32),
    Y4(i32),
    Z0,
    Z1(i8),
    Z2(i16),
    Z3(i32),
    Z4(i32),
    FntNumN(u8),
    Fnt1(u8),
    Fnt2(u16),
    Fnt3(u32),
    Fnt4(i32),
    XXX1(Vec<u8>),
    XXX2(Vec<u8>),
    XXX3(Vec<u8>),
    XXX4(Vec<u8>),
    FntDef1 {
        font_num: u8,
        checksum: u32,
        scale: u32,
        design_size: u32,
        area: u8,
        length: u8,
        font_name: String,
    },
    FntDef2 {
        font_num: u16,
        checksum: u32,
        scale: u32,
        design_size: u32,
        area: u8,
        length: u8,
        font_name: String,
    },
    FntDef3 {
        font_num: u32,
        checksum: u32,
        scale: u32,
        design_size: u32,
        area: u8,
        length: u8,
        font_name: String,
    },
    FntDef4 {
        font_num: i32,
        checksum: u32,
        scale: u32,
        design_size: u32,
        area: u8,
        length: u8,
        font_name: String,
    },
    Pre {
        format: u8,
        num: u32,
        den: u32,
        mag: u32,
        comment: Vec<u8>,
    },
    Post {
        pointer: u32,
        num: u32,
        den: u32,
        mag: u32,
        max_page_height: u32,
        max_page_width: u32,
        max_stack_depth: u16,
        num_pages: u16,
    },
    PostPost {
        post_pointer: u32,
        format: u8,
        tail: u8,
    },
}

impl DVICommand {
    pub fn byte_size(&self) -> usize {
        match self {
            DVICommand::SetCharN(_) => 1,
            DVICommand::Set1(_) => 2,
            DVICommand::Set2(_) => 3,
            DVICommand::Set3(_) => 4,
            DVICommand::Set4(_) => 5,
            DVICommand::SetRule {
                height: _,
                width: _,
            } => 9,
            DVICommand::Put1(_) => 2,
            DVICommand::Put2(_) => 3,
            DVICommand::Put3(_) => 4,
            DVICommand::Put4(_) => 5,
            DVICommand::PutRule {
                height: _,
                width: _,
            } => 9,
            DVICommand::Nop => 1,
            DVICommand::Bop { cs: _, pointer: _ } => 45,
            DVICommand::Eop => 1,
            DVICommand::Push => 1,
            DVICommand::Pop => 1,
            DVICommand::Right1(_) => 2,
            DVICommand::Right2(_) => 3,
            DVICommand::Right3(_) => 4,
            DVICommand::Right4(_) => 5,
            DVICommand::W0 => 1,
            DVICommand::W1(_) => 2,
            DVICommand::W2(_) => 3,
            DVICommand::W3(_) => 4,
            DVICommand::W4(_) => 5,
            DVICommand::X0 => 1,
            DVICommand::X1(_) => 2,
            DVICommand::X2(_) => 3,
            DVICommand::X3(_) => 4,
            DVICommand::X4(_) => 5,
            DVICommand::Down1(_) => 2,
            DVICommand::Down2(_) => 3,
            DVICommand::Down3(_) => 4,
            DVICommand::Down4(_) => 5,
            DVICommand::Y0 => 1,
            DVICommand::Y1(_) => 2,
            DVICommand::Y2(_) => 3,
            DVICommand::Y3(_) => 4,
            DVICommand::Y4(_) => 5,
            DVICommand::Z0 => 1,
            DVICommand::Z1(_) => 2,
            DVICommand::Z2(_) => 3,
            DVICommand::Z3(_) => 4,
            DVICommand::Z4(_) => 5,
            DVICommand::FntNumN(_) => 1,
            DVICommand::Fnt1(_) => 2,
            DVICommand::Fnt2(_) => 3,
            DVICommand::Fnt3(_) => 4,
            DVICommand::Fnt4(_) => 5,
            DVICommand::XXX1(v) => 2 + v.len(),
            DVICommand::XXX2(v) => 3 + v.len(),
            DVICommand::XXX3(v) => 4 + v.len(),
            DVICommand::XXX4(v) => 5 + v.len(),
            DVICommand::FntDef1 {
                font_num: _,
                checksum: _,
                scale: _,
                design_size: _,
                area: _,
                length: _,
                font_name,
            } => 16 + font_name.len(),
            DVICommand::FntDef2 {
                font_num: _,
                checksum: _,
                scale: _,
                design_size: _,
                area: _,
                length: _,
                font_name,
            } => 17 + font_name.len(),
            DVICommand::FntDef3 {
                font_num: _,
                checksum: _,
                scale: _,
                design_size: _,
                area: _,
                length: _,
                font_name,
            } => 18 + font_name.len(),
            DVICommand::FntDef4 {
                font_num: _,
                checksum: _,
                scale: _,
                design_size: _,
                area: _,
                length: _,
                font_name,
            } => 19 + font_name.len(),
            DVICommand::Pre {
                format: _,
                num: _,
                den: _,
                mag: _,
                comment,
            } => 15 + comment.len(),
            DVICommand::Post {
                pointer: _,
                num: _,
                den: _,
                mag: _,
                max_page_height: _,
                max_page_width: _,
                max_stack_depth: _,
                num_pages: _,
            } => 29,
            DVICommand::PostPost {
                post_pointer: _,
                format: _,
                tail,
            } => 6 + (*tail as usize),
        }
    }
}

/// A faithful representation of a DVI file, consisting of the DVI commands.
/// Does no interpretation of the contents of the file, so this could contain
/// invalid constructs.
#[derive(Debug, PartialEq)]
pub struct DVIFile {
    pub commands: Vec<DVICommand>,
}

#[derive(Debug, PartialEq)]
struct TFMHeader {
    checksum: u32,
    design_size: f64,
    coding_scheme: String,
    parc_font_identifier: String,
    seven_bit_safe: bool,
    parc_face_byte: u8,
}

#[derive(Debug, PartialEq, Eq)]
enum CharKind {
    Vanilla,
    LigKern { ligkern_index: usize },
    CharList { next_char: u8 },
    Extensible { ext_recipe_index: usize },
}

#[derive(Debug, PartialEq, Eq)]
struct CharInfoEntry {
    width_index: usize,
    height_index: usize,
    depth_index: usize,
    italic_correction_index: usize,
    kind: CharKind,
}

#[derive(Debug, PartialEq, Eq)]
enum LigKernKind {
    Ligature { substitution: usize },
    Kern { kern_index: usize },
}

#[derive(Debug, PartialEq, Eq)]
struct LigKernStep {
    stop: bool,
    next_char: usize,
    kind: LigKernKind,
}

#[derive(Debug, PartialEq, Eq)]
struct ExtRecipe {
    top: usize,
    mid: usize,
    bot: usize,
    ext: usize,
}

#[derive(Debug, PartialEq)]
pub struct TFMFile {
    first_char: usize,
    last_char: usize,

    header: TFMHeader,

    char_infos: Vec<CharInfoEntry>,
    widths: Vec<f64>,
    heights: Vec<f64>,
    depths: Vec<f64>,
    italic_corrections: Vec<f64>,
    lig_kern_steps: Vec<LigKernStep>,
    kerns: Vec<f64>,
    ext_recipes: Vec<ExtRecipe>,
    font_parameters: Vec<f64>,
}

mod accessors;
mod file_reader;
mod read_tfm;

#[cfg(test)]
mod test_data;

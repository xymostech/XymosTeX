use crate::boxes::TeXBox;
use crate::dimension::Dimen;
use crate::math_code::{MathClass, MathCode};

#[derive(Debug, PartialEq)]
pub enum AtomKind {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    #[allow(dead_code)]
    Inner,
    #[allow(dead_code)]
    Over,
    #[allow(dead_code)]
    Under,
    #[allow(dead_code)]
    Acc,
    #[allow(dead_code)]
    Rad,
    #[allow(dead_code)]
    Vcent,
}

impl AtomKind {
    fn from_math_class(class: &MathClass) -> AtomKind {
        match class {
            MathClass::Ordinary => AtomKind::Ord,
            MathClass::LargeOperator => AtomKind::Op,
            MathClass::BinaryOperation => AtomKind::Bin,
            MathClass::Relation => AtomKind::Rel,
            MathClass::Opening => AtomKind::Open,
            MathClass::Closing => AtomKind::Close,
            MathClass::Punctuation => AtomKind::Punct,
            MathClass::VariableFamily => AtomKind::Ord,
            MathClass::Active => panic!("invalid atom kind: active"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MathSymbol {
    family_number: u8,
    position_number: u8,
}

impl MathSymbol {
    pub fn from_math_code(math_code: &MathCode) -> MathSymbol {
        MathSymbol {
            // TODO: check if the class is VariableFamily, in which case we
            // need to check \fam
            family_number: math_code.family,
            position_number: math_code.position,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MathField {
    Symbol(MathSymbol),
    #[allow(dead_code)]
    TeXBox(TeXBox),
    MathList(MathList),
}

#[derive(Debug, PartialEq)]
pub struct MathAtom {
    kind: AtomKind,
    nucleus: Option<MathField>,
    superscript: Option<MathField>,
    subscript: Option<MathField>,
}

impl MathAtom {
    pub fn from_math_code(math_code: &MathCode) -> MathAtom {
        let symbol = MathSymbol::from_math_code(math_code);

        MathAtom {
            kind: AtomKind::from_math_class(&math_code.class),
            nucleus: Some(MathField::Symbol(symbol)),
            superscript: None,
            subscript: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum MathStyle {
    DisplayStyle,
    DisplayStylePrime,
    TextStyle,
    TextStylePrime,
    ScriptStyle,
    ScriptStylePrime,
    ScriptScriptStyle,
    ScriptScriptStylePrime,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct MathDelimiter {
    small_font_family: u16,
    small_position: u16,
    large_font_family: u16,
    large_position: u16,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct GeneralizedFraction {
    left_delim: Option<MathDelimiter>,
    right_delim: Option<MathDelimiter>,
    bar_height: Dimen,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum BoundaryKind {
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum MathListElem {
    Atom(MathAtom),
    #[allow(dead_code)]
    StyleChange(MathStyle),
    #[allow(dead_code)]
    GeneralizedFraction(GeneralizedFraction),
    #[allow(dead_code)]
    Boundary(BoundaryKind, Option<MathDelimiter>),
    #[allow(dead_code)]
    FourWayChoice {
        display: MathList,
        text: MathList,
        script: MathList,
        scriptscript: MathList,
    },
}

pub type MathList = Vec<MathListElem>;

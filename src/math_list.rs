use crate::boxes::TeXBox;
use crate::dimension::Dimen;
use crate::math_code::{MathClass, MathCode};

#[derive(Debug, PartialEq)]
enum AtomKind {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
    Over,
    Under,
    Acc,
    Rad,
    Vcent,
}

impl AtomKind {
    fn from_math_class(class: MathClass) -> AtomKind {
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
struct MathSymbol {
    family_number: u8,
    position_number: u8,
}

#[derive(Debug, PartialEq)]
enum AtomField {
    Symbol(MathSymbol),
    TeXBox(TeXBox),
    MathList(MathList),
}

#[derive(Debug, PartialEq)]
pub struct MathAtom {
    kind: AtomKind,
    nucleus: Option<AtomField>,
    superscript: Option<AtomField>,
    subscript: Option<AtomField>,
}

impl MathAtom {
    pub fn from_math_code(math_code: MathCode) -> MathAtom {
        let symbol = MathSymbol {
            // TODO: check if the class is VariableFamily, in which case we
            // need to check \fam
            family_number: math_code.family,
            position_number: math_code.position,
        };

        MathAtom {
            kind: AtomKind::from_math_class(math_code.class),
            nucleus: Some(AtomField::Symbol(symbol)),
            superscript: None,
            subscript: None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum MathStyle {
    DisplayStyle,
    DisplayStylePrime,
    TextStyle,
    TextStylePrime,
    ScriptStyle,
    ScriptStylePrime,
    ScriptScriptStyle,
    ScriptScriptStylePrime,
}

#[derive(Debug, PartialEq)]
struct MathDelimiter {
    smallFontFamily: u16,
    smallPosition: u16,
    largeFontFamily: u16,
    largePosition: u16,
}

#[derive(Debug, PartialEq)]
struct GeneralizedFraction {
    leftDelim: Option<MathDelimiter>,
    rightDelim: Option<MathDelimiter>,
    barHeight: Dimen,
}

#[derive(Debug, PartialEq)]
enum BoundaryKind {
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum MathListElem {
    Atom(MathAtom),
    StyleChange(MathStyle),
    GeneralizedFraction(GeneralizedFraction),
    Boundary(BoundaryKind, Option<MathDelimiter>),
    FourWayChoice {
        display: MathList,
        text: MathList,
        script: MathList,
        scriptscript: MathList,
    },
}

pub type MathList = Vec<MathListElem>;

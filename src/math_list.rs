use crate::boxes::TeXBox;
use crate::dimension::Dimen;
use crate::math_code::{MathClass, MathCode};

#[derive(Debug, PartialEq, Clone, Hash, Eq, Copy)]
pub enum AtomKind {
    Ord,   // 0
    Op,    // 1
    Bin,   // 2
    Rel,   // 3
    Open,  // 4
    Close, // 5
    Punct, // 6
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
    pub family_number: u8,
    pub position_number: u8,
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
    pub kind: AtomKind,
    pub nucleus: Option<MathField>,
    pub superscript: Option<MathField>,
    pub subscript: Option<MathField>,
}

impl MathAtom {
    pub fn empty_ord() -> MathAtom {
        MathAtom {
            kind: AtomKind::Ord,
            nucleus: None,
            superscript: None,
            subscript: None,
        }
    }

    pub fn from_math_code(math_code: &MathCode) -> MathAtom {
        let symbol = MathSymbol::from_math_code(math_code);

        MathAtom {
            kind: AtomKind::from_math_class(&math_code.class),
            nucleus: Some(MathField::Symbol(symbol)),
            superscript: None,
            subscript: None,
        }
    }

    pub fn from_math_list(math_list: MathList) -> MathAtom {
        MathAtom {
            kind: AtomKind::Ord,
            nucleus: Some(MathField::MathList(math_list)),
            superscript: None,
            subscript: None,
        }
    }

    pub fn from_box(tex_box: TeXBox) -> MathAtom {
        MathAtom {
            kind: AtomKind::Ord,
            nucleus: Some(MathField::TeXBox(tex_box)),
            superscript: None,
            subscript: None,
        }
    }

    pub fn with_superscript(mut self, superscript: MathField) -> MathAtom {
        self.superscript = Some(superscript);
        self
    }

    pub fn has_superscript(&self) -> bool {
        self.superscript.is_some()
    }

    pub fn with_subscript(mut self, subscript: MathField) -> MathAtom {
        self.subscript = Some(subscript);
        self
    }

    pub fn has_subscript(&self) -> bool {
        self.subscript.is_some()
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
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

impl MathStyle {
    pub fn is_script(&self) -> bool {
        match *self {
            MathStyle::DisplayStyle => false,
            MathStyle::DisplayStylePrime => false,
            MathStyle::TextStyle => false,
            MathStyle::TextStylePrime => false,
            MathStyle::ScriptStyle => true,
            MathStyle::ScriptStylePrime => true,
            MathStyle::ScriptScriptStyle => true,
            MathStyle::ScriptScriptStylePrime => true,
        }
    }

    pub fn prime(&self) -> MathStyle {
        match *self {
            MathStyle::DisplayStyle => MathStyle::DisplayStylePrime,
            MathStyle::DisplayStylePrime => MathStyle::DisplayStylePrime,
            MathStyle::TextStyle => MathStyle::TextStylePrime,
            MathStyle::TextStylePrime => MathStyle::TextStylePrime,
            MathStyle::ScriptStyle => MathStyle::ScriptStylePrime,
            MathStyle::ScriptStylePrime => MathStyle::ScriptStylePrime,
            MathStyle::ScriptScriptStyle => MathStyle::ScriptScriptStylePrime,
            MathStyle::ScriptScriptStylePrime => {
                MathStyle::ScriptScriptStylePrime
            }
        }
    }

    pub fn up_arrow(&self) -> MathStyle {
        match *self {
            MathStyle::DisplayStyle => MathStyle::ScriptStyle,
            MathStyle::DisplayStylePrime => MathStyle::ScriptStylePrime,
            MathStyle::TextStyle => MathStyle::ScriptStyle,
            MathStyle::TextStylePrime => MathStyle::ScriptStylePrime,
            MathStyle::ScriptStyle => MathStyle::ScriptScriptStyle,
            MathStyle::ScriptStylePrime => MathStyle::ScriptScriptStylePrime,
            MathStyle::ScriptScriptStyle => MathStyle::ScriptScriptStyle,
            MathStyle::ScriptScriptStylePrime => {
                MathStyle::ScriptScriptStylePrime
            }
        }
    }

    pub fn down_arrow(&self) -> MathStyle {
        self.up_arrow().prime()
    }
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

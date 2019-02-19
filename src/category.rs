#[allow(dead_code)] // TODO: remove this once all of these are used
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Category {
    Escape,       // 0
    BeginGroup,   // 1
    EndGroup,     // 2
    MathShift,    // 3
    AlignmentTab, // 4
    EndOfLine,    // 5
    Parameter,    // 6
    Superscript,  // 7
    Subscript,    // 8
    Ignored,      // 9
    Space,        // 10
    Letter,       // 11
    Other,        // 12
    Active,       // 13
    Comment,      // 14
    Invalid,      // 15
}

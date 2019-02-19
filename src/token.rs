use crate::category::Category;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Token {
    ControlSequence(String),
    Char(char, Category),
}

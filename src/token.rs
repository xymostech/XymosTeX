use crate::category::Category;

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    ControlSequence(String),
    Char(char, Category),
}

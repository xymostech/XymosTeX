use crate::category::Category;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    ControlSequence(String),
    Char(char, Category),
}

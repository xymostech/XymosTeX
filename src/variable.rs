use crate::state::TeXState;

#[derive(PartialEq, Eq, Debug)]
pub enum Variable {
    Count(u8),
}

impl Variable {
    pub fn set(&self, state: &TeXState, global: bool, value: i32) {
        match self {
            Variable::Count(index) => state.set_count(global, *index, value),
        }
    }

    pub fn get(&self, state: &TeXState) -> i32 {
        match self {
            Variable::Count(index) => state.get_count(*index),
        }
    }
}

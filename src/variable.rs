use crate::state::TeXState;

#[derive(PartialEq, Eq, Debug)]
pub enum IntegerVariable {
    CountRegister(u8),
}

impl IntegerVariable {
    pub fn set(&self, state: &TeXState, global: bool, value: i32) {
        match self {
            IntegerVariable::CountRegister(index) => {
                state.set_count(global, *index, value)
            }
        }
    }

    pub fn get(&self, state: &TeXState) -> i32 {
        match self {
            IntegerVariable::CountRegister(index) => state.get_count(*index),
        }
    }
}

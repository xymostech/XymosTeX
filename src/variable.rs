use crate::dimension::Dimen;
use crate::state::TeXState;

#[derive(PartialEq, Eq, Debug)]
pub enum IntegerVariable {
    CountRegister(u8),
}

impl IntegerVariable {
    pub fn set(&self, state: &TeXState, global: bool, value: i32) {
        match self {
            Self::CountRegister(index) => {
                state.set_count(global, *index, value)
            }
        }
    }

    pub fn get(&self, state: &TeXState) -> i32 {
        match self {
            Self::CountRegister(index) => state.get_count(*index),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum DimenVariable {
    BoxWidth(u8),
    BoxHeight(u8),
    BoxDepth(u8),
}

impl DimenVariable {
    pub fn get(&self, state: &TeXState) -> Dimen {
        match self {
            Self::BoxWidth(index) => state
                .with_box(*index, |tex_box| *tex_box.width())
                .unwrap_or_else(Dimen::zero),
            Self::BoxHeight(index) => state
                .with_box(*index, |tex_box| *tex_box.height())
                .unwrap_or_else(Dimen::zero),
            Self::BoxDepth(index) => state
                .with_box(*index, |tex_box| *tex_box.depth())
                .unwrap_or_else(Dimen::zero),
        }
    }

    pub fn set(&self, state: &TeXState, _global: bool, new_dimen: Dimen) {
        match self {
            Self::BoxWidth(index) => {
                state.with_box(*index, |tex_box| {
                    *tex_box.mut_width() = new_dimen
                });
            }
            Self::BoxHeight(index) => {
                state.with_box(*index, |tex_box| {
                    *tex_box.mut_height() = new_dimen
                });
            }
            Self::BoxDepth(index) => {
                state.with_box(*index, |tex_box| {
                    *tex_box.mut_depth() = new_dimen
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::{HorizontalBox, TeXBox};
    use crate::dimension::Unit;

    #[test]
    fn it_gets_box_dimen_variables() {
        let state = TeXState::new();

        let width_variable = DimenVariable::BoxWidth(123);
        let height_variable = DimenVariable::BoxHeight(123);
        let depth_variable = DimenVariable::BoxDepth(123);
        assert_eq!(width_variable.get(&state), Dimen::zero());

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(1.0, Unit::Point),
            depth: Dimen::from_unit(2.0, Unit::Point),
            width: Dimen::from_unit(3.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        state.set_box(false, 123, test_box);
        assert_eq!(
            height_variable.get(&state),
            Dimen::from_unit(1.0, Unit::Point)
        );
        assert_eq!(
            depth_variable.get(&state),
            Dimen::from_unit(2.0, Unit::Point)
        );
        assert_eq!(
            width_variable.get(&state),
            Dimen::from_unit(3.0, Unit::Point)
        );
    }

    #[test]
    fn it_sets_box_dimen_variables() {
        let state = TeXState::new();

        let width_variable = DimenVariable::BoxWidth(123);
        let height_variable = DimenVariable::BoxHeight(123);
        let depth_variable = DimenVariable::BoxDepth(123);
        width_variable.set(&state, false, Dimen::from_unit(1.0, Unit::Point));

        let test_box = TeXBox::HorizontalBox(HorizontalBox {
            height: Dimen::from_unit(1.0, Unit::Point),
            depth: Dimen::from_unit(2.0, Unit::Point),
            width: Dimen::from_unit(3.0, Unit::Point),
            list: Vec::new(),
            glue_set_ratio: None,
        });

        state.set_box(false, 123, test_box.clone());
        state.push_state();
        state.set_box(false, 123, test_box);

        // We set the new dimensions globally to test that this actually only
        // affects the top-level boxes
        height_variable.set(&state, true, Dimen::from_unit(4.0, Unit::Point));
        depth_variable.set(&state, true, Dimen::from_unit(5.0, Unit::Point));
        width_variable.set(&state, true, Dimen::from_unit(6.0, Unit::Point));

        // The dimensions of the top-level box are changed
        let mut final_inner_box = state.get_box(123).unwrap();
        assert_eq!(
            *final_inner_box.mut_height(),
            Dimen::from_unit(4.0, Unit::Point)
        );
        assert_eq!(
            *final_inner_box.mut_depth(),
            Dimen::from_unit(5.0, Unit::Point)
        );
        assert_eq!(
            *final_inner_box.mut_width(),
            Dimen::from_unit(6.0, Unit::Point)
        );

        state.pop_state();

        // The dimensions of the lower-level box are not changed
        let mut final_outer_box = state.get_box(123).unwrap();
        assert_eq!(
            *final_outer_box.mut_height(),
            Dimen::from_unit(1.0, Unit::Point)
        );
        assert_eq!(
            *final_outer_box.mut_depth(),
            Dimen::from_unit(2.0, Unit::Point)
        );
        assert_eq!(
            *final_outer_box.mut_width(),
            Dimen::from_unit(3.0, Unit::Point)
        );
    }
}

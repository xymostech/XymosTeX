use crate::dimension::Dimen;
use crate::glue::Glue;
use crate::state::{DimenParameter, GlueParameter, IntegerParameter, TeXState};

#[derive(PartialEq, Eq, Debug)]
pub enum IntegerVariable {
    CountRegister(u8),
    Pretolerance,
    Tolerance,
    TracingParagraphs,
}

impl IntegerVariable {
    pub fn set(&self, state: &TeXState, global: bool, value: i32) {
        match self {
            Self::CountRegister(index) => {
                state.set_count(global, *index, value)
            }
            Self::Pretolerance => state.set_integer_parameter(
                global,
                &IntegerParameter::Pretolerance,
                value,
            ),
            Self::Tolerance => state.set_integer_parameter(
                global,
                &IntegerParameter::Tolerance,
                value,
            ),
            Self::TracingParagraphs => state.set_integer_parameter(
                global,
                &IntegerParameter::TracingParagraphs,
                value,
            ),
        }
    }

    pub fn get(&self, state: &TeXState) -> i32 {
        match self {
            Self::CountRegister(index) => state.get_count(*index),
            Self::Pretolerance => {
                state.get_integer_parameter(&IntegerParameter::Pretolerance)
            }
            Self::Tolerance => {
                state.get_integer_parameter(&IntegerParameter::Tolerance)
            }
            Self::TracingParagraphs => state
                .get_integer_parameter(&IntegerParameter::TracingParagraphs),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum DimenVariable {
    BoxWidth(u8),
    BoxHeight(u8),
    BoxDepth(u8),
    HSize,
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
            Self::HSize => state.get_dimen_parameter(&DimenParameter::HSize),
        }
    }

    pub fn set(&self, state: &TeXState, global: bool, new_dimen: Dimen) {
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
            Self::HSize => state.set_dimen_parameter(
                global,
                &DimenParameter::HSize,
                &new_dimen,
            ),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum GlueVariable {
    ParSkip,
    SpaceSkip,
    ParFillSkip,
}

impl GlueVariable {
    pub fn get(&self, state: &TeXState) -> Glue {
        match self {
            Self::ParSkip => state.get_glue_parameter(&GlueParameter::ParSkip),
            Self::SpaceSkip => {
                state.get_glue_parameter(&GlueParameter::SpaceSkip)
            }
            Self::ParFillSkip => {
                state.get_glue_parameter(&GlueParameter::ParFillSkip)
            }
        }
    }

    pub fn set(&self, state: &TeXState, global: bool, new_glue: Glue) {
        match self {
            Self::ParSkip => state.set_glue_parameter(
                global,
                &GlueParameter::ParSkip,
                &new_glue,
            ),
            Self::SpaceSkip => state.set_glue_parameter(
                global,
                &GlueParameter::SpaceSkip,
                &new_glue,
            ),
            Self::ParFillSkip => state.set_glue_parameter(
                global,
                &GlueParameter::ParFillSkip,
                &new_glue,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::boxes::{HorizontalBox, TeXBox};
    use crate::dimension::{SpringDimen, Unit};

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

    #[test]
    fn it_gets_and_sets_dimen_parameters() {
        let state = TeXState::new();

        let param_variable = DimenVariable::HSize;

        assert_eq!(
            param_variable.get(&state),
            Dimen::from_unit(6.5, Unit::Inch)
        );

        // Local assignment
        state.push_state();
        param_variable.set(&state, false, Dimen::from_unit(10.0, Unit::Point));

        assert_eq!(
            param_variable.get(&state),
            Dimen::from_unit(10.0, Unit::Point)
        );

        state.pop_state();

        assert_eq!(
            param_variable.get(&state),
            Dimen::from_unit(6.5, Unit::Inch)
        );

        // Global assignment
        state.push_state();
        param_variable.set(&state, true, Dimen::from_unit(50.0, Unit::Point));

        assert_eq!(
            param_variable.get(&state),
            Dimen::from_unit(50.0, Unit::Point)
        );

        state.pop_state();

        assert_eq!(
            param_variable.get(&state),
            Dimen::from_unit(50.0, Unit::Point)
        );
    }

    #[test]
    fn it_gets_and_sets_glue_parameters() {
        let state = TeXState::new();

        let parskip_variable = GlueVariable::ParSkip;

        let default_value = Glue {
            space: Dimen::zero(),
            stretch: SpringDimen::Dimen(Dimen::from_unit(1.0, Unit::Point)),
            shrink: SpringDimen::Dimen(Dimen::zero()),
        };
        let new_value = Glue::from_dimen(Dimen::from_unit(2.0, Unit::Point));

        assert_eq!(parskip_variable.get(&state), default_value);

        state.push_state();
        parskip_variable.set(&state, false, new_value.clone());
        assert_eq!(parskip_variable.get(&state), new_value);

        state.pop_state();
        assert_eq!(parskip_variable.get(&state), default_value);

        state.push_state();
        parskip_variable.set(&state, true, new_value.clone());
        state.pop_state();
        assert_eq!(parskip_variable.get(&state), new_value);
    }

    #[test]
    fn it_gets_and_sets_integer_parameters() {
        let state = TeXState::new();

        let tolerance = IntegerVariable::Tolerance;
        let pretolerance = IntegerVariable::Pretolerance;

        assert_eq!(tolerance.get(&state), 200);
        assert_eq!(pretolerance.get(&state), 100);

        tolerance.set(&state, false, 1000);
        assert_eq!(tolerance.get(&state), 1000);

        state.push_state();
        pretolerance.set(&state, true, 500);
        assert_eq!(pretolerance.get(&state), 500);

        state.pop_state();
        assert_eq!(pretolerance.get(&state), 500);
    }
}

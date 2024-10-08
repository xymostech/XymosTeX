use crate::parser::Parser;
use crate::state::{DimenParameter, GlueParameter, IntegerParameter};
use crate::variable::{DimenVariable, GlueVariable, IntegerVariable};

impl<'a> Parser<'a> {
    pub fn is_integer_variable_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "count",
            "tolerance",
            "pretolerance",
            "tracingparagraphs",
            "adjdemerits",
        ])
    }

    pub fn parse_integer_variable(&mut self) -> IntegerVariable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "count") {
            let index = self.parse_8bit_number();
            IntegerVariable::CountRegister(index)
        } else if self.state.is_token_equal_to_prim(&token, "tolerance") {
            IntegerVariable::Parameter(IntegerParameter::Tolerance)
        } else if self.state.is_token_equal_to_prim(&token, "pretolerance") {
            IntegerVariable::Parameter(IntegerParameter::Pretolerance)
        } else if self
            .state
            .is_token_equal_to_prim(&token, "tracingparagraphs")
        {
            IntegerVariable::Parameter(IntegerParameter::TracingParagraphs)
        } else if self.state.is_token_equal_to_prim(&token, "adjdemerits") {
            IntegerVariable::Parameter(IntegerParameter::AdjDemerits)
        } else {
            panic!("unimplemented");
        }
    }

    pub fn is_dimen_variable_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "wd", "ht", "dp", "hsize",
        ])
    }

    pub fn parse_dimen_variable(&mut self) -> DimenVariable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "wd") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxWidth(index)
        } else if self.state.is_token_equal_to_prim(&token, "ht") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxHeight(index)
        } else if self.state.is_token_equal_to_prim(&token, "dp") {
            let index = self.parse_8bit_number();
            DimenVariable::BoxDepth(index)
        } else if self.state.is_token_equal_to_prim(&token, "hsize") {
            DimenVariable::Parameter(DimenParameter::HSize)
        } else {
            panic!("unimplemented");
        }
    }

    pub fn is_glue_variable_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "parskip",
            "spaceskip",
            "parfillskip",
        ])
    }

    pub fn parse_glue_variable(&mut self) -> GlueVariable {
        let token = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&token, "parskip") {
            GlueVariable::Parameter(GlueParameter::ParSkip)
        } else if self.state.is_token_equal_to_prim(&token, "spaceskip") {
            GlueVariable::Parameter(GlueParameter::SpaceSkip)
        } else if self.state.is_token_equal_to_prim(&token, "parfillskip") {
            GlueVariable::Parameter(GlueParameter::ParFillSkip)
        } else {
            panic!("unimplemented");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::with_parser;

    #[test]
    fn it_parses_count_variables() {
        with_parser(
            &["\\let\\x=\\count%", "\\count0%", "\\count255%", "\\x255%"],
            |parser| {
                parser.parse_assignment(None);

                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(0)
                );
                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(255)
                );
                assert!(parser.is_integer_variable_head());
                assert_eq!(
                    parser.parse_integer_variable(),
                    IntegerVariable::CountRegister(255)
                );
            },
        );
    }

    #[test]
    fn it_parses_box_dimen_variables() {
        with_parser(&["\\wd0%", "\\ht255%", "\\dp123%"], |parser| {
            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxWidth(0)
            );

            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxHeight(255)
            );

            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::BoxDepth(123)
            );
        });
    }

    #[test]
    fn it_parses_other_dimen_variables() {
        with_parser(&["\\hsize%"], |parser| {
            assert!(parser.is_dimen_variable_head());
            assert_eq!(
                parser.parse_dimen_variable(),
                DimenVariable::Parameter(DimenParameter::HSize)
            );
        });
    }

    #[test]
    fn it_parses_glue_parameter_variables() {
        with_parser(&[r"\parskip%", r"\spaceskip%"], |parser| {
            assert!(parser.is_glue_variable_head());
            assert_eq!(
                parser.parse_glue_variable(),
                GlueVariable::Parameter(GlueParameter::ParSkip),
            );

            assert!(parser.is_glue_variable_head());
            assert_eq!(
                parser.parse_glue_variable(),
                GlueVariable::Parameter(GlueParameter::SpaceSkip),
            );
        });
    }

    #[test]
    fn it_parses_integer_parameter_variables() {
        with_parser(&[r"\tolerance%", r"\pretolerance%"], |parser| {
            assert!(parser.is_integer_variable_head());
            assert_eq!(
                parser.parse_integer_variable(),
                IntegerVariable::Parameter(IntegerParameter::Tolerance)
            );

            assert!(parser.is_integer_variable_head());
            assert_eq!(
                parser.parse_integer_variable(),
                IntegerVariable::Parameter(IntegerParameter::Pretolerance)
            );
        });
    }
}

use crate::parser::Parser;

use crate::dimension::{Dimen, SpringDimen};
use crate::glue::Glue;

impl<'a> Parser<'a> {
    fn is_internal_glue_head(&mut self) -> bool {
        self.is_glue_variable_head()
    }

    fn parse_internal_glue(&mut self) -> Glue {
        if self.is_glue_variable_head() {
            let variable = self.parse_glue_variable();
            variable.get(self.state)
        } else {
            panic!("unimplemented");
        }
    }

    fn parse_normal_glue(&mut self, sign: i32) -> Glue {
        let space = self.parse_dimen() * sign;

        let mut stretch = SpringDimen::Dimen(Dimen::zero());
        let mut shrink = SpringDimen::Dimen(Dimen::zero());

        if self.parse_optional_keyword_expanded("plus") {
            stretch = self.parse_spring_dimen(true);
        }

        if self.parse_optional_keyword_expanded("minus") {
            shrink = self.parse_spring_dimen(true);
        }

        Glue {
            space,
            stretch,
            shrink,
        }
    }

    pub fn parse_glue(&mut self) -> Glue {
        let sign = self.parse_optional_signs();

        if self.is_internal_glue_head() {
            let glue = self.parse_internal_glue();
            glue * sign
        } else {
            self.parse_normal_glue(sign)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::category::Category;
    use crate::dimension::Unit;
    use crate::state::GlueParameter;
    use crate::testing::with_parser;
    use crate::token::Token;

    #[test]
    fn it_parses_glue_without_stretch_and_shrink() {
        with_parser(&["1pt %"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );
        });
    }

    #[test]
    fn it_parses_glue_without_shrink() {
        with_parser(&["1pt plus 2pt %"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::from_unit(
                        2.0,
                        Unit::Point
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );
        });
    }

    #[test]
    fn it_parses_glue_without_stretch() {
        with_parser(&["1pt minus 3pt %"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::from_unit(
                        3.0,
                        Unit::Point
                    )),
                }
            );
        });
    }

    #[test]
    fn it_parses_glue_with_stretch_and_shrink() {
        with_parser(&["1pt plus 2pt minus 3pt %"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::from_unit(
                        2.0,
                        Unit::Point
                    )),
                    shrink: SpringDimen::Dimen(Dimen::from_unit(
                        3.0,
                        Unit::Point
                    )),
                }
            );
        });
    }

    #[test]
    fn it_doesnt_fail_when_seeing_a_partial_keyword() {
        with_parser(&["1pt plu%", "1pt plus 2pt minu%"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );

            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('p', Category::Letter))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('l', Category::Letter))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('u', Category::Letter))
            );

            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(1.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::from_unit(
                        2.0,
                        Unit::Point
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );

            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('m', Category::Letter))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('i', Category::Letter))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('n', Category::Letter))
            );
            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('u', Category::Letter))
            );
        });
    }

    #[test]
    fn it_parses_plain_dimens_with_negative_signs() {
        with_parser(&[r"-3pt plus 3pt%", "-- -  -- -4pt%"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(-3.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::from_unit(
                        3.0,
                        Unit::Point
                    )),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );
            assert_eq!(
                parser.parse_glue(),
                Glue {
                    space: Dimen::from_unit(4.0, Unit::Point),
                    stretch: SpringDimen::Dimen(Dimen::zero()),
                    shrink: SpringDimen::Dimen(Dimen::zero()),
                }
            );
        });
    }

    #[test]
    fn it_parses_glue_variables() {
        with_parser(&[r"\parskip%", r"- -- --\parskip%"], |parser| {
            assert_eq!(
                parser.parse_glue(),
                parser.state.get_glue_parameter(&GlueParameter::ParSkip)
            );
            assert_eq!(
                parser.parse_glue(),
                parser.state.get_glue_parameter(&GlueParameter::ParSkip) * -1
            );
        });
    }
}

use crate::category::Category;
use crate::dimension::{Dimen, Unit};
use crate::parser::number::{is_token_digit, token_digit_value};
use crate::parser::primitives::token_equals_keyword_char;
use crate::parser::Parser;
use crate::token::Token;

fn is_decimal_point_token(token: &Token) -> bool {
    match token {
        Token::Char(ch, Category::Other) => *ch == ',' || *ch == '.',
        _ => false,
    }
}

fn equals_unit(
    first_token: &Token,
    second_token: &Token,
    unit: [char; 2],
) -> bool {
    token_equals_keyword_char(first_token, unit[0])
        && token_equals_keyword_char(second_token, unit[1])
}

enum ParsedUnit {
    Em,
    Ex,
    PhysicalUnit(bool, Unit),
}

// Since we don't have a real \mag parameter yet, we just use this constant
// when parsing "true" dimensions.
const MAG_FACTOR: i32 = 1000;

impl<'a> Parser<'a> {
    fn parse_dimen(&mut self) -> Dimen {
        let sign = self.parse_optional_signs();
        let value = self.parse_unsigned_dimen();

        value * sign
    }

    fn parse_unsigned_dimen(&mut self) -> Dimen {
        self.parse_normal_dimen()
    }

    fn parse_normal_dimen(&mut self) -> Dimen {
        let factor = self.parse_factor();
        let (unit_factor, unit) = self.parse_unit_of_measure();

        Dimen::from_unit(factor * unit_factor, unit)
    }

    fn is_almost_normal_integer_head(&mut self) -> bool {
        self.is_internal_integer_head()
    }

    // Parses a <normal integer> except without the integer constant, because
    // we handle a superset of the integer constant parsing in parsing a
    // decimal constant.
    fn parse_almost_normal_integer(&mut self) -> i32 {
        if self.is_internal_integer_head() {
            self.parse_internal_integer()
        } else {
            panic!("unimplemented")
        }
    }

    fn is_decimal_constant_head(&mut self) -> bool {
        match self.peek_expanded_token() {
            Some(token) => {
                is_token_digit(&token) || is_decimal_point_token(&token)
            }
            _ => false,
        }
    }

    // Parses a decimal constant, which is any number of digits and a single
    // decimal point (either . or ,).
    fn parse_decimal_constant(&mut self) -> f64 {
        // The final decimal value
        let mut value: f64 = 0.0;

        // Whether or not we've seen a decimal point yet
        let mut seen_decimal_point = false;

        // After we've seen a decimal point, each new digit contributes 1/10
        // the value of the previous digit to the final value. This stores the
        // multiplier for the next digit we see.
        let mut decimal_factor: f64 = 1.0 / 10.0;

        // Keep track of whether we've seen any digits or a decimal point at
        // all, because if we don't see any digits or a decimal point, then we
        // should fail parsing.
        let mut seen_any_digits = false;

        loop {
            match self.peek_expanded_token() {
                Some(ref token) if is_token_digit(token) => {
                    self.lex_expanded_token();
                    let token_value = token_digit_value(token) as f64;
                    if seen_decimal_point {
                        // If we've already seen a decimal point, then add to
                        // our final value the new value times the current
                        // factor.
                        value = value + token_value * decimal_factor;
                        // And then the next digit should have 1/10 the impact,
                        // so devide our factor by 10.
                        decimal_factor /= 10.0;
                    } else {
                        // If we haven't seen a decimal point yet, then add
                        // this new digit at the end of the integer part.
                        value = value * 10.0 + token_value;
                    }
                    seen_any_digits = true;
                }
                Some(ref token)
                    if is_decimal_point_token(token) && !seen_decimal_point =>
                {
                    // If we see a decimal point (and haven't seen a decimal
                    // point yet), keep track of that, which will affect how we
                    // handle new digits.
                    self.lex_expanded_token();
                    seen_decimal_point = true;
                    seen_any_digits = true;
                }
                _ => break,
            }
        }

        if !seen_any_digits {
            panic!("No digits found while parsing decimal constant");
        }

        value
    }

    // Parses a <factor> (which is the number part of a <number><unit>
    // dimension).
    fn parse_factor(&mut self) -> f64 {
        if self.is_almost_normal_integer_head() {
            self.parse_almost_normal_integer() as f64
        } else if self.is_decimal_constant_head() {
            self.parse_decimal_constant()
        } else {
            panic!("unimplemented");
        }
    }

    // Parses a "unit of measure". Returns a tuple of (factor, unit). We can't
    // just return a unit because:
    // * Sometimes a "unit of measure" is actually a variable, in which case
    //   we're probably going to end up with (<number>, ScaledPoint) as we turn
    //   the variable into a dimen
    // * Sometimes, the unit we're using isn't a physical unit, but something
    //   like an em which depends on the current font we're using.
    // * Sometimes, we find a "true" unit, which depends on the current
    //   magnification (from \mag)
    fn parse_unit_of_measure(&mut self) -> (f64, Unit) {
        if self.is_internal_integer_head() {
            let value = self.parse_internal_integer();
            (value as f64, Unit::ScaledPoint)
        } else {
            match self.parse_unit() {
                ParsedUnit::PhysicalUnit(is_true, unit) => {
                    if is_true {
                        // TODO(xymostech): Lookup the \mag factor from the
                        // state instead of just using a constant.
                        (1000.0 / (MAG_FACTOR as f64), unit)
                    } else {
                        (1.0, unit)
                    }
                }
                // These need font metrics to be looked up before we can
                // correctly get the values for this.
                ParsedUnit::Em => panic!("unimplemented"),
                ParsedUnit::Ex => panic!("unimplemented"),
            }
        }
    }

    fn parse_unit(&mut self) -> ParsedUnit {
        self.parse_optional_spaces_expanded();

        let mut is_true_unit = false;

        // Check to see if our unit starts with a 't', in which case we're
        // parsing a 'true' unit (because none of the other units start with
        // 't').
        let true_start = self.peek_expanded_token().unwrap();
        if token_equals_keyword_char(&true_start, 't') {
            // We're seeing a 'true' at the start of the unit. Parse that.
            self.parse_keyword_expanded("true");
            self.parse_optional_spaces_expanded();

            is_true_unit = true;
        }

        // Since all of the units have 2 characters, just parse both character
        // tokens up front.
        let unit_first = self.lex_expanded_token().unwrap();
        let unit_second = self.lex_expanded_token().unwrap();

        self.parse_optional_space_expanded();

        // Check to see which unit the first and second tokens match.
        if equals_unit(&unit_first, &unit_second, ['p', 't']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Point)
        } else if equals_unit(&unit_first, &unit_second, ['p', 'c']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Pica)
        } else if equals_unit(&unit_first, &unit_second, ['i', 'n']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Inch)
        } else if equals_unit(&unit_first, &unit_second, ['b', 'p']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::BigPoint)
        } else if equals_unit(&unit_first, &unit_second, ['c', 'm']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Centimeter)
        } else if equals_unit(&unit_first, &unit_second, ['m', 'm']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Millimeter)
        } else if equals_unit(&unit_first, &unit_second, ['d', 'd']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::DidotPoint)
        } else if equals_unit(&unit_first, &unit_second, ['c', 'c']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::Cicero)
        } else if equals_unit(&unit_first, &unit_second, ['s', 'p']) {
            ParsedUnit::PhysicalUnit(is_true_unit, Unit::ScaledPoint)
        } else if equals_unit(&unit_first, &unit_second, ['e', 'm']) {
            if is_true_unit {
                panic!("Invalid unit with true: em");
            }
            ParsedUnit::Em
        } else if equals_unit(&unit_first, &unit_second, ['e', 'x']) {
            if is_true_unit {
                panic!("Invalid unit with true: ex");
            }
            ParsedUnit::Ex
        } else {
            panic!("Invalid unit: {:?}{:?}%", unit_first, unit_second);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::with_parser;

    #[test]
    fn it_parses_basic_units() {
        with_parser(
            &[
                "1pt%", "1pc%", "1in%", "1bp%", "1cm%", "1mm%", "1dd%", "1cc%",
                "1sp%",
            ],
            |parser| {
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Point)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Pica)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Inch)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::BigPoint)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Centimeter)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Millimeter)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::DidotPoint)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::Cicero)
                );
                assert_eq!(
                    parser.parse_dimen(),
                    Dimen::from_unit(1.0, Unit::ScaledPoint)
                );
            },
        );
    }

    #[test]
    fn it_parses_decimals() {
        with_parser(
            &[
                "1 %", "1. %", "1.0 %", "1234 %", "1.2 %", ".2 %", ".234 %",
                "1,23 %",
            ],
            |parser| {
                assert_eq!(parser.parse_decimal_constant(), 1.0);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 1.0);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 1.0);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 1234.0);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 1.2);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 0.2);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 0.234);
                parser.parse_optional_spaces_expanded();
                assert_eq!(parser.parse_decimal_constant(), 1.23);
                parser.parse_optional_spaces_expanded();
            },
        );
    }

    #[test]
    fn it_parses_integer_constants_after_decimals() {
        with_parser(&["3.4\\count0%"], |parser| {
            parser.state.set_count(false, 0, 123);
            assert_eq!(
                parser.parse_dimen(),
                Dimen::from_unit(3.4 * 123.0, Unit::ScaledPoint)
            );
        });
    }

    #[test]
    fn it_parses_integer_constants_before_unit() {
        with_parser(&["\\count0 cc%"], |parser| {
            parser.state.set_count(false, 0, 123);
            assert_eq!(
                parser.parse_dimen(),
                Dimen::from_unit(123.0, Unit::Cicero)
            );
        });
    }

    #[test]
    fn it_parses_true_units() {
        with_parser(&["3.4 true pt%"], |parser| {
            assert_eq!(
                parser.parse_dimen(),
                Dimen::from_unit(3.4, Unit::Point)
            );
        });
    }

    #[test]
    fn it_parses_negative_dimens() {
        with_parser(&["-3.4pt%"], |parser| {
            assert_eq!(
                parser.parse_dimen(),
                Dimen::from_unit(-3.4, Unit::Point)
            );
        });
    }

    #[test]
    fn it_parses_optional_spaces_after_units() {
        with_parser(&["1pt %"], |parser| {
            parser.parse_dimen();
            assert_eq!(parser.lex_unexpanded_token(), None);
        });
    }
}

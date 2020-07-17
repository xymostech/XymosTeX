use crate::category::Category;
use crate::math_code::MathCode;
use crate::math_list::{MathAtom, MathList, MathListElem};
use crate::parser::Parser;
use crate::token::Token;

impl<'a> Parser<'a> {
    fn is_character_head(&mut self) -> bool {
        let expanded_token = self.peek_expanded_token();
        match self.replace_renamed_token(expanded_token) {
            Some(Token::Char(_, Category::Letter)) => true,
            Some(Token::Char(_, Category::Other)) => true,
            _ => false,
        }
    }

    fn parse_character_to_math_code(&mut self) -> MathCode {
        let expanded_token = self.lex_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);

        let ch: char = match expanded_renamed_token {
            Some(Token::Char(ch, _)) => ch,
            _ => panic!(),
        };

        self.state.get_math_code(ch)
    }

    fn is_math_symbol_head(&mut self) -> bool {
        self.is_character_head()
    }

    fn parse_math_symbol(&mut self) -> MathCode {
        if self.is_character_head() {
            self.parse_character_to_math_code()
        } else {
            panic!("Unimplemented");
        }
    }

    fn parse_math_list(&mut self) -> MathList {
        let mut current_list = Vec::new();

        loop {
            if self.peek_expanded_token() == None {
                break;
            } else if self.is_math_symbol_head() {
                let math_code = self.parse_math_symbol();

                current_list.push(MathListElem::Atom(
                    MathAtom::from_math_code(math_code),
                ));
            } else {
                panic!("unimplemented");
            }
        }

        current_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::with_parser;

    #[test]
    fn it_parses_math_symbols() {
        with_parser(&["a2*%"], |parser| {
            assert_eq!(
                parser.parse_math_symbol(),
                MathCode::from_number(0x7161)
            );
            assert_eq!(
                parser.parse_math_symbol(),
                MathCode::from_number(0x7032)
            );
            assert_eq!(
                parser.parse_math_symbol(),
                MathCode::from_number(0x002a)
            );
        });
    }

    #[test]
    fn it_parses_math_symbols_from_chardefs() {
        with_parser(&[r"\let\x=z%", r"\x%"], |parser| {
            parser.parse_assignment();

            assert_eq!(
                parser.parse_math_symbol(),
                MathCode::from_number(0x717a)
            );
        });
    }

    #[test]
    fn it_parses_basic_atoms_in_math_lists() {
        with_parser(&[r"a*%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(MathAtom::from_math_code(
                        MathCode::from_number(0x7161)
                    )),
                    MathListElem::Atom(MathAtom::from_math_code(
                        MathCode::from_number(0x002a)
                    )),
                ]
            );
        });
    }
}

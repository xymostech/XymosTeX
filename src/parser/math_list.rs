use crate::category::Category;
use crate::math_code::MathCode;
use crate::math_list::{
    MathAtom, MathField, MathList, MathListElem, MathStyle, MathSymbol,
};
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

    fn parse_math_group(&mut self) -> MathList {
        let begin_group = self.lex_expanded_token();
        match begin_group {
            Some(Token::Char(_, Category::BeginGroup)) => (),
            tok => panic!("Invalid start of math group: {:?}", tok),
        }

        let math_list = self.parse_math_list();

        let end_group = self.lex_expanded_token();
        match end_group {
            Some(Token::Char(_, Category::EndGroup)) => (),
            tok => panic!("Math group didn't end with an EndGroup: {:?}", tok),
        }

        math_list
    }

    fn parse_math_field(&mut self) -> MathField {
        self.parse_filler_expanded();

        if self.is_math_symbol_head() {
            let math_code = self.parse_math_symbol();

            MathField::Symbol(MathSymbol::from_math_code(&math_code))
        } else {
            MathField::MathList(self.parse_math_group())
        }
    }

    fn is_math_superscript_head(&mut self) -> bool {
        let expanded_token = self.peek_expanded_token();
        match self.replace_renamed_token(expanded_token) {
            Some(Token::Char(_, Category::Superscript)) => true,
            _ => false,
        }
    }

    fn parse_math_superscript(&mut self, atom: MathAtom) -> MathAtom {
        self.lex_expanded_token();

        if atom.has_superscript() {
            panic!("Double superscript");
        }

        let superscript = self.parse_math_field();
        atom.with_superscript(superscript)
    }

    fn parse_math_list(&mut self) -> MathList {
        let mut current_list = Vec::new();

        loop {
            if self.is_math_symbol_head() {
                let math_code = self.parse_math_symbol();

                current_list.push(MathListElem::Atom(
                    MathAtom::from_math_code(&math_code),
                ));
            } else if self.is_math_superscript_head() {
                let last_atom = match current_list.pop() {
                    Some(MathListElem::Atom(atom)) => atom,
                    Some(other_elem) => {
                        current_list.push(other_elem);
                        MathAtom::empty_ord()
                    }
                    None => MathAtom::empty_ord(),
                };

                current_list.push(MathListElem::Atom(
                    self.parse_math_superscript(last_atom),
                ));
            } else {
                match self.peek_expanded_token() {
                    Some(Token::Char(_, Category::EndGroup)) => break,
                    None => break,
                    _ => panic!("unimplemented"),
                }
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
                        &MathCode::from_number(0x7161)
                    )),
                    MathListElem::Atom(MathAtom::from_math_code(
                        &MathCode::from_number(0x002a)
                    )),
                ]
            );
        });
    }

    #[test]
    fn it_parses_basic_math_groups() {
        with_parser(&[r"{a}%"], |parser| {
            assert_eq!(
                parser.parse_math_group(),
                vec![MathListElem::Atom(MathAtom::from_math_code(
                    &MathCode::from_number(0x7161)
                )),],
            );
        });
    }

    #[test]
    #[should_panic(expected = "Invalid start of math group")]
    fn it_fails_parsing_math_groups_not_starting_with_begin_group() {
        with_parser(&[r"a%"], |parser| {
            parser.parse_math_group();
        });
    }

    #[test]
    #[should_panic(expected = "Math group didn't end with an EndGroup")]
    fn it_fails_parsing_math_groups_not_ending_with_end_group() {
        with_parser(&[r"{a%"], |parser| {
            parser.parse_math_group();
        });
    }

    #[test]
    fn it_parses_symbols_as_math_fields() {
        with_parser(&[r"a2%"], |parser| {
            assert_eq!(
                parser.parse_math_field(),
                MathField::Symbol(MathSymbol::from_math_code(
                    &MathCode::from_number(0x7161)
                ))
            );
            assert_eq!(
                parser.parse_math_field(),
                MathField::Symbol(MathSymbol::from_math_code(
                    &MathCode::from_number(0x7032)
                ))
            );
        })
    }

    #[test]
    fn it_parses_groups_as_math_fields() {
        with_parser(&[r"{ab}{}%"], |parser| {
            assert_eq!(
                parser.parse_math_field(),
                MathField::MathList(vec![
                    MathListElem::Atom(MathAtom::from_math_code(
                        &MathCode::from_number(0x7161)
                    )),
                    MathListElem::Atom(MathAtom::from_math_code(
                        &MathCode::from_number(0x7162)
                    )),
                ],)
            );
            assert_eq!(parser.parse_math_field(), MathField::MathList(vec![],));
        });
    }

    #[test]
    fn it_ignores_filler_before_math_fields() {
        with_parser(&[r"  a   {a}%"], |parser| {
            assert_eq!(
                parser.parse_math_field(),
                MathField::Symbol(MathSymbol::from_math_code(
                    &MathCode::from_number(0x7161)
                ))
            );
            assert_eq!(
                parser.parse_math_field(),
                MathField::MathList(vec![MathListElem::Atom(
                    MathAtom::from_math_code(&MathCode::from_number(0x7161))
                ),],)
            );
        });
    }

    #[test]
    fn it_parses_superscripts() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);

        with_parser(&[r"a^a%", r"a^{ab}%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(
                        MathAtom::from_math_code(&a_code).with_superscript(
                            MathField::Symbol(MathSymbol::from_math_code(
                                &a_code
                            ))
                        )
                    ),
                    MathListElem::Atom(
                        MathAtom::from_math_code(&a_code).with_superscript(
                            MathField::MathList(vec![
                                MathListElem::Atom(MathAtom::from_math_code(
                                    &a_code
                                )),
                                MathListElem::Atom(MathAtom::from_math_code(
                                    &b_code
                                )),
                            ])
                        )
                    )
                ],
            );
        });
    }

    #[test]
    fn it_parses_superscripts_at_beginning_of_lists() {
        let a_code = MathCode::from_number(0x7161);

        with_parser(&[r"^a%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::Atom(
                    MathAtom::empty_ord().with_superscript(MathField::Symbol(
                        MathSymbol::from_math_code(&a_code)
                    ))
                ),],
            );
        });
    }

    #[test]
    #[should_panic(expected = "Double superscript")]
    fn it_fails_on_multiple_superscripts() {
        with_parser(&[r"a^a^a%"], |parser| {
            parser.parse_math_list();
        });
    }

    #[test]
    // \displaystyle isn't parsed yet so this test won't work, but we want to
    // test this case in the future so ignore this test for now.
    #[ignore]
    fn it_parses_superscripts_after_non_atoms() {
        let a_code = MathCode::from_number(0x7161);

        with_parser(&[r"\displaystyle ^{a}%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::StyleChange(MathStyle::DisplayStyle),
                    MathListElem::Atom(MathAtom::empty_ord().with_superscript(
                        MathField::Symbol(MathSymbol::from_math_code(&a_code))
                    )),
                ],
            );
        });
    }
}

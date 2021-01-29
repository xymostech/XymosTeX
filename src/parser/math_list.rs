use crate::boxes::{HorizontalBox, TeXBox};
use crate::category::Category;
use crate::list::HorizontalListElem;
use crate::math_code::MathCode;
use crate::math_list::{
    AtomKind, MathAtom, MathField, MathList, MathListElem, MathStyle,
    MathSymbol,
};
use crate::parser::boxes::BoxLayout;
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

    fn is_math_character_head(&mut self) -> bool {
        let expanded_token = self.peek_expanded_token();
        if let Some(expanded_renamed_token) =
            self.replace_renamed_token(expanded_token)
        {
            self.state
                .get_math_chardef(&expanded_renamed_token)
                .is_some()
        } else {
            false
        }
    }

    fn parse_math_character_to_math_code(&mut self) -> MathCode {
        let expanded_token = self.lex_expanded_token();
        let expanded_renamed_token =
            self.replace_renamed_token(expanded_token).unwrap();

        if let Some(math_code) =
            self.state.get_math_chardef(&expanded_renamed_token)
        {
            math_code
        } else {
            panic!("Invalid math chardef token: {:?}", expanded_renamed_token);
        }
    }

    fn is_math_symbol_head(&mut self) -> bool {
        self.is_character_head() || self.is_math_character_head()
    }

    fn parse_math_symbol(&mut self) -> MathCode {
        if self.is_character_head() {
            self.parse_character_to_math_code()
        } else if self.is_math_character_head() {
            self.parse_math_character_to_math_code()
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

        self.state.push_state();

        let math_list = self.parse_math_list();

        self.state.pop_state();

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

    fn is_math_subscript_head(&mut self) -> bool {
        let expanded_token = self.peek_expanded_token();
        match self.replace_renamed_token(expanded_token) {
            Some(Token::Char(_, Category::Subscript)) => true,
            _ => false,
        }
    }

    fn parse_math_subscript(&mut self, atom: MathAtom) -> MathAtom {
        self.lex_expanded_token();

        if atom.has_subscript() {
            panic!("Double subscript");
        }

        let subscript = self.parse_math_field();
        atom.with_subscript(subscript)
    }

    fn parse_math_list(&mut self) -> MathList {
        let mut current_list = Vec::new();

        loop {
            if self.is_math_symbol_head() {
                let math_code = self.parse_math_symbol();

                current_list.push(MathListElem::Atom(
                    MathAtom::from_math_code(&math_code),
                ));
            } else if self.is_math_superscript_head()
                || self.is_math_subscript_head()
            {
                let is_superscript = self.is_math_superscript_head();

                let last_atom = match current_list.pop() {
                    Some(MathListElem::Atom(atom)) => atom,
                    Some(other_elem) => {
                        current_list.push(other_elem);
                        MathAtom::empty_ord()
                    }
                    None => MathAtom::empty_ord(),
                };

                current_list.push(MathListElem::Atom(if is_superscript {
                    self.parse_math_superscript(last_atom)
                } else {
                    self.parse_math_subscript(last_atom)
                }));
            } else if self.is_assignment_head() {
                self.parse_assignment();
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

    fn convert_math_list_to_horizontal_list(
        &mut self,
        list: MathList,
    ) -> Vec<HorizontalListElem> {
        let mut elems_after_first_pass: MathList = Vec::new();

        for mut elem in list {
            match elem {
                MathListElem::Atom(mut atom) => {
                    match atom.nucleus {
                        Some(MathField::Symbol(symbol)) => {
                            let char_elem = HorizontalListElem::Char {
                                chr: symbol.position_number as char,
                                // TODO figure out what goes here
                                font: self.state.get_current_font(),
                            };

                            let hbox = self
                                .add_to_natural_layout_horizontal_box(
                                    HorizontalBox::empty(),
                                    char_elem,
                                );

                            atom.nucleus = Some(MathField::TeXBox(
                                TeXBox::HorizontalBox(hbox),
                            ));
                        }
                        Some(MathField::TeXBox(_)) => {
                            // Nothing to do
                        }
                        Some(MathField::MathList(list)) => {
                            let hlist =
                                self.convert_math_list_to_horizontal_list(list);
                            let hbox = self.combine_horizontal_list_into_horizontal_box_with_layout(hlist, &BoxLayout::Natural);

                            atom.nucleus = Some(MathField::TeXBox(
                                TeXBox::HorizontalBox(hbox),
                            ));
                        }
                        None => {}
                    }

                    if atom.has_subscript() || atom.has_superscript() {
                        panic!("Unimplemented superscript/subscript");
                    }

                    elems_after_first_pass.push(MathListElem::Atom(atom));
                }
                _ => {
                    panic!("unimplemented math list elem: {:?}", elem);
                }
            }
        }

        let mut resulting_horizontal_list: Vec<HorizontalListElem> = Vec::new();

        for elem in elems_after_first_pass {
            match elem {
                MathListElem::Atom(atom) => {
                    if atom.has_subscript() || atom.has_superscript() {
                        panic!("Atoms should be sub/superscript free in second pass!");
                    }

                    match atom.nucleus {
                        Some(MathField::TeXBox(texbox)) => {
                            resulting_horizontal_list
                                .push(HorizontalListElem::Box(texbox));
                        }
                        None => {}
                        _ => {
                            panic!("Atom nucleuses should only be boxes in second pass!");
                        }
                    }
                }
                _ => {
                    panic!("unimplemented math list elem: {:?}");
                }
            }
        }

        resulting_horizontal_list
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
    fn it_scopes_assignments_in_math_fields() {
        with_parser(&[r"\count 0=1%", r"a^{\count 0=2}%"], |parser| {
            parser.parse_math_list();

            assert_eq!(parser.state.get_count(0), 1);
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
    #[should_panic(expected = "Double superscript")]
    fn it_fails_on_multiple_superscripts_after_subscript() {
        with_parser(&[r"a^a_a^a%"], |parser| {
            parser.parse_math_list();
        });
    }

    #[test]
    fn it_parses_subscripts() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);

        with_parser(&[r"a_a%", r"a_{ab}%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(
                        MathAtom::from_math_code(&a_code).with_subscript(
                            MathField::Symbol(MathSymbol::from_math_code(
                                &a_code
                            ))
                        )
                    ),
                    MathListElem::Atom(
                        MathAtom::from_math_code(&a_code).with_subscript(
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
    fn it_parses_subscripts_at_beginning_of_lists() {
        let a_code = MathCode::from_number(0x7161);

        with_parser(&[r"_a%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::Atom(MathAtom::empty_ord().with_subscript(
                    MathField::Symbol(MathSymbol::from_math_code(&a_code))
                )),],
            );
        });
    }

    #[test]
    #[should_panic(expected = "Double subscript")]
    fn it_fails_on_multiple_subscripts() {
        with_parser(&[r"a_a_a%"], |parser| {
            parser.parse_math_list();
        });
    }

    #[test]
    #[should_panic(expected = "Double subscript")]
    fn it_fails_on_multiple_subscripts_after_superscript() {
        with_parser(&[r"a_a^a_a%"], |parser| {
            parser.parse_math_list();
        });
    }

    #[test]
    fn it_parses_mathchardefs() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);
        let c_code = MathCode::from_number(0x7163);

        with_parser(&[r"\hello%", r"a\hello b%"], |parser| {
            let tok = parser.lex_unexpanded_token().unwrap();
            parser.state.set_math_chardef(false, &tok, &c_code);

            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                ],
            );
        });
    }

    #[test]
    fn it_parses_assignments_in_math_mode() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);
        let c_code = MathCode::from_number(0x7163);

        with_parser(&[r"a\def\x #1{a#1b}%", r"b\x c%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                ]
            );
        });

        with_parser(&[r"a\def\x{b}_\x%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::Atom(
                    MathAtom::from_math_code(&a_code).with_subscript(
                        MathField::Symbol(MathSymbol::from_math_code(&b_code))
                    )
                ),]
            );
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

    #[test]
    fn it_produces_empty_horizontal_lists_from_empty_math_lists() {
        with_parser(&[r"%"], |parser| {
            let math_list = parser.parse_math_list();
            assert_eq!(
                parser.convert_math_list_to_horizontal_list(math_list),
                vec![]
            );
        });
    }

    #[test]
    fn it_produces_single_characters_from_single_atom_math_lists() {
        with_parser(&[r"\hbox{a}a%"], |parser| {
            let hbox = parser.parse_box().unwrap();
            let math_list = parser.parse_math_list();
            assert_eq!(
                parser.convert_math_list_to_horizontal_list(math_list),
                vec![HorizontalListElem::Box(hbox)]
            );
        });
    }

    #[test]
    fn it_produces_multiple_characters_from_multiple_ord_math_lists() {
        with_parser(&[r"\hbox{a}\hbox{b}ab%"], |parser| {
            let hbox_a = parser.parse_box().unwrap();
            let hbox_b = parser.parse_box().unwrap();
            let math_list = parser.parse_math_list();
            assert_eq!(
                parser.convert_math_list_to_horizontal_list(math_list),
                vec![
                    HorizontalListElem::Box(hbox_a),
                    HorizontalListElem::Box(hbox_b)
                ]
            );
        });
    }
}

use std::cmp::{max, Ordering};
use std::collections::HashMap;

use crate::boxes::{HorizontalBox, TeXBox, VerticalBox};
use crate::category::Category;
use crate::dimension::{Dimen, FilDimen, FilKind, SpringDimen, Unit};
use crate::font::Font;
use crate::glue::Glue;
use crate::list::{HorizontalListElem, VerticalListElem};
use crate::math_code::MathCode;
use crate::math_list::{
    AtomKind, GeneralizedFraction, MathAtom, MathDelimiter, MathField,
    MathList, MathListElem, MathStyle, MathSymbol,
};
use crate::parser::boxes::BoxLayout;
use crate::parser::Parser;
use crate::token::Token;

#[derive(Clone)]
enum InterAtomSpacing {
    None,
    ThinSkip,
    ThinSkipNonScript,
    MediumSkipNonScript,
    ThickSkipNonScript,
}

lazy_static! {
    static ref INTER_ATOM_SPACING: HashMap<(AtomKind, AtomKind), InterAtomSpacing> = [
        // 0 1 (2) (3) 0 0 0 (1)
        ((AtomKind::Ord, AtomKind::Ord), InterAtomSpacing::None),
        ((AtomKind::Ord, AtomKind::Op), InterAtomSpacing::ThinSkip),
        ((AtomKind::Ord, AtomKind::Bin), InterAtomSpacing::MediumSkipNonScript),
        ((AtomKind::Ord, AtomKind::Rel), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Ord, AtomKind::Open), InterAtomSpacing::None),
        ((AtomKind::Ord, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Ord, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Ord, AtomKind::Inner), InterAtomSpacing::ThinSkipNonScript),

        // 1 1 * (3) 0 0 0 (1)
        ((AtomKind::Op, AtomKind::Ord), InterAtomSpacing::ThinSkip),
        ((AtomKind::Op, AtomKind::Op), InterAtomSpacing::ThinSkip),
        //((AtomKind::Op, AtomKind::Bin), InterAtomSpacing::None),
        ((AtomKind::Op, AtomKind::Rel), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Op, AtomKind::Open), InterAtomSpacing::None),
        ((AtomKind::Op, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Op, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Op, AtomKind::Inner), InterAtomSpacing::ThinSkipNonScript),

        // (2) (2) * * (2) * * (2)
        ((AtomKind::Bin, AtomKind::Ord), InterAtomSpacing::MediumSkipNonScript),
        ((AtomKind::Bin, AtomKind::Op), InterAtomSpacing::MediumSkipNonScript),
        //((AtomKind::Bin, AtomKind::Bin), InterAtomSpacing::None),
        //((AtomKind::Bin, AtomKind::Rel), InterAtomSpacing::None),
        ((AtomKind::Bin, AtomKind::Open), InterAtomSpacing::MediumSkipNonScript),
        //((AtomKind::Bin, AtomKind::Close), InterAtomSpacing::None),
        //((AtomKind::Bin, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Bin, AtomKind::Inner), InterAtomSpacing::MediumSkipNonScript),

        // (3) (3) * 0 (3) 0 0 (3)
        ((AtomKind::Rel, AtomKind::Ord), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Rel, AtomKind::Op), InterAtomSpacing::ThickSkipNonScript),
        //((AtomKind::Rel, AtomKind::Bin), InterAtomSpacing::None),
        ((AtomKind::Rel, AtomKind::Rel), InterAtomSpacing::None),
        ((AtomKind::Rel, AtomKind::Open), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Rel, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Rel, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Rel, AtomKind::Inner), InterAtomSpacing::ThickSkipNonScript),

        // 0 0 * 0 0 0 0 0
        ((AtomKind::Open, AtomKind::Ord), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Op), InterAtomSpacing::None),
        //((AtomKind::Open, AtomKind::Bin), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Rel), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Open), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Open, AtomKind::Inner), InterAtomSpacing::None),

        // 0 1 (2) (3) 0 0 0 (1)
        ((AtomKind::Close, AtomKind::Ord), InterAtomSpacing::None),
        ((AtomKind::Close, AtomKind::Op), InterAtomSpacing::ThinSkip),
        ((AtomKind::Close, AtomKind::Bin), InterAtomSpacing::MediumSkipNonScript),
        ((AtomKind::Close, AtomKind::Rel), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Close, AtomKind::Open), InterAtomSpacing::None),
        ((AtomKind::Close, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Close, AtomKind::Punct), InterAtomSpacing::None),
        ((AtomKind::Close, AtomKind::Inner), InterAtomSpacing::ThinSkipNonScript),

        // (1) (1) * (1) (1) (1) (1) (1)
        ((AtomKind::Punct, AtomKind::Ord), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Punct, AtomKind::Op), InterAtomSpacing::ThinSkipNonScript),
        //((AtomKind::Punct, AtomKind::Bin), InterAtomSpacing::None),
        ((AtomKind::Punct, AtomKind::Rel), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Punct, AtomKind::Open), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Punct, AtomKind::Close), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Punct, AtomKind::Punct), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Punct, AtomKind::Inner), InterAtomSpacing::ThinSkipNonScript),

        // (1) 1 (2) (3) (1) 0 (1) (1)
        ((AtomKind::Inner, AtomKind::Ord), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Inner, AtomKind::Op), InterAtomSpacing::ThinSkip),
        ((AtomKind::Inner, AtomKind::Bin), InterAtomSpacing::MediumSkipNonScript),
        ((AtomKind::Inner, AtomKind::Rel), InterAtomSpacing::ThickSkipNonScript),
        ((AtomKind::Inner, AtomKind::Open), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Inner, AtomKind::Close), InterAtomSpacing::None),
        ((AtomKind::Inner, AtomKind::Punct), InterAtomSpacing::ThinSkipNonScript),
        ((AtomKind::Inner, AtomKind::Inner), InterAtomSpacing::ThinSkipNonScript),
    ].iter().cloned().collect();
}

fn get_font_style_for_math_style(style: &MathStyle) -> MathStyle {
    match style {
        MathStyle::DisplayStyle => MathStyle::TextStyle,
        MathStyle::DisplayStylePrime => MathStyle::TextStyle,
        MathStyle::TextStyle => MathStyle::TextStyle,
        MathStyle::TextStylePrime => MathStyle::TextStyle,
        MathStyle::ScriptStyle => MathStyle::ScriptStyle,
        MathStyle::ScriptStylePrime => MathStyle::ScriptStyle,
        MathStyle::ScriptScriptStyle => MathStyle::ScriptScriptStyle,
        MathStyle::ScriptScriptStylePrime => MathStyle::ScriptScriptStyle,
    }
}

lazy_static! {
    // TODO: pull these from \textfont, \scriptfont, and \scriptscriptfont
    static ref MATH_FONTS: HashMap<(MathStyle, u8), Font> = [
        ((MathStyle::TextStyle, 0), Font {
            font_name: "cmr10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
        ((MathStyle::ScriptStyle, 0), Font {
            font_name: "cmr7".to_string(),
            scale: Dimen::from_unit(7.0, Unit::Point),
        }),
        ((MathStyle::ScriptScriptStyle, 0), Font {
            font_name: "cmr5".to_string(),
            scale: Dimen::from_unit(5.0, Unit::Point),
        }),
        ((MathStyle::TextStyle, 1), Font {
            font_name: "cmmi10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
        ((MathStyle::ScriptStyle, 1), Font {
            font_name: "cmmi7".to_string(),
            scale: Dimen::from_unit(7.0, Unit::Point),
        }),
        ((MathStyle::ScriptScriptStyle, 1), Font {
            font_name: "cmmi5".to_string(),
            scale: Dimen::from_unit(5.0, Unit::Point),
        }),
        ((MathStyle::TextStyle, 2), Font {
            font_name: "cmsy10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
        ((MathStyle::ScriptStyle, 2), Font {
            font_name: "cmsy7".to_string(),
            scale: Dimen::from_unit(7.0, Unit::Point),
        }),
        ((MathStyle::ScriptScriptStyle, 2), Font {
            font_name: "cmsy5".to_string(),
            scale: Dimen::from_unit(5.0, Unit::Point),
        }),
        ((MathStyle::TextStyle, 3), Font {
            font_name: "cmex10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
        ((MathStyle::ScriptStyle, 3), Font {
            font_name: "cmex10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
        ((MathStyle::ScriptScriptStyle, 3), Font {
            font_name: "cmex10".to_string(),
            scale: Dimen::from_unit(10.0, Unit::Point),
        }),
    ].iter().cloned().collect();
}

struct TranslatedNucleus {
    translation: Vec<HorizontalListElem>,
    nucleus_is_symbol: bool,
    effective_height: Dimen,
    effective_depth: Dimen,
}

// This represents the translation of a given MathAtom into horizontal list
// elems.
struct TranslatedMathAtom {
    kind: AtomKind,
    translation: Vec<HorizontalListElem>,
}

// When performing the transformation from math list to horizontal list,
// there's an intermediate step where not everything has been translated, but
// the result is no longer a plain MathList. This type keeps track of all of
// the necessary elements in that intermediate step.
enum TranslatedMathListElem {
    Atom(TranslatedMathAtom),
    StyleChange(MathStyle),
}

impl<'a> Parser<'a> {
    fn is_character_head(&mut self) -> bool {
        let expanded_token = self.peek_expanded_token();

        match self.replace_renamed_token(expanded_token) {
            Some(Token::Char(_, Category::Letter)) => true,
            Some(Token::Char(_, Category::Other)) => true,
            Some(tok) => self.state.is_token_equal_to_prim(&tok, "char"),
            _ => false,
        }
    }

    fn parse_character_to_math_code(&mut self) -> MathCode {
        let expanded_token = self.lex_expanded_token();
        let expanded_renamed_token = self.replace_renamed_token(expanded_token);

        let ch: char = match expanded_renamed_token {
            Some(Token::Char(ch, _)) => ch,
            Some(tok) => {
                if self.state.is_token_equal_to_prim(&tok, "char") {
                    let char_number = self.parse_8bit_number();
                    char_number as char
                } else {
                    panic!("invalid char token head");
                }
            }
            _ => panic!("invalid char token head"),
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

    fn is_style_change_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "displaystyle",
            "textstyle",
            "scriptstyle",
            "scriptscriptstyle",
        ])
    }

    fn parse_style_change(&mut self) -> MathStyle {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&tok, "displaystyle") {
            MathStyle::DisplayStyle
        } else if self.state.is_token_equal_to_prim(&tok, "textstyle") {
            MathStyle::TextStyle
        } else if self.state.is_token_equal_to_prim(&tok, "scriptstyle") {
            MathStyle::ScriptStyle
        } else if self.state.is_token_equal_to_prim(&tok, "scriptscriptstyle") {
            MathStyle::ScriptScriptStyle
        } else {
            panic!("Invalid style change");
        }
    }

    fn is_generalized_fraction_head(&mut self) -> bool {
        self.is_next_expanded_token_in_set_of_primitives(&[
            "over",
            "atop",
            "above",
            "overwithdelims",
            "atopwithdelims",
            "abovewithdelims",
        ])
    }

    fn parse_generalized_fraction_params(
        &mut self,
    ) -> (Option<MathDelimiter>, Option<MathDelimiter>, Dimen) {
        let tok = self.lex_expanded_token().unwrap();

        if self.state.is_token_equal_to_prim(&tok, "atop") {
            (None, None, Dimen::zero())
        } else {
            panic!("unimplemented");
        }
    }

    pub fn parse_math_list(&mut self) -> MathList {
        let mut current_list = Vec::new();

        // Keep track of whether there's been a generalized fraction operation
        // within this math list by storing the intermediate numerator of the
        // list as well as the generalized fraction parameters here.
        let mut list_fraction = None;

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
                self.parse_assignment(None);
            } else if self.is_style_change_head() {
                let style_change = self.parse_style_change();
                current_list.push(MathListElem::StyleChange(style_change));
            } else if self.is_box_head() {
                if let Some(tex_box) = self.parse_box() {
                    current_list
                        .push(MathListElem::Atom(MathAtom::from_box(tex_box)));
                }
            } else if self.is_generalized_fraction_head() {
                if list_fraction.is_some() {
                    panic!("Ambiguous generalized fraction");
                }

                let (
                    gen_frac_left_delim,
                    gen_frac_right_delim,
                    gen_frac_height,
                ) = self.parse_generalized_fraction_params();

                list_fraction = Some(GeneralizedFraction {
                    left_delim: gen_frac_left_delim,
                    right_delim: gen_frac_right_delim,
                    bar_height: gen_frac_height,
                    numerator: current_list,
                    denominator: vec![],
                });
                current_list = vec![];
            } else {
                match self.peek_expanded_token() {
                    Some(Token::Char(_, Category::BeginGroup)) => {
                        let inner_list = self.parse_math_group();
                        current_list.push(MathListElem::Atom(
                            MathAtom::from_math_list(inner_list),
                        ));
                    }
                    Some(Token::Char(_, Category::Space)) => {
                        self.lex_expanded_token();
                    }
                    Some(Token::Char(_, Category::EndGroup)) => break,
                    Some(Token::Char(_, Category::MathShift)) => break,
                    None => break,
                    _ => panic!("unimplemented"),
                }
            }
        }

        match list_fraction {
            None => current_list,
            Some(mut fraction) => {
                fraction.denominator = current_list;
                vec![MathListElem::GeneralizedFraction(fraction)]
            }
        }
    }

    fn get_skip_for_atom_pair(
        &mut self,
        left_type: &AtomKind,
        right_type: &AtomKind,
        style: &MathStyle,
    ) -> Option<Glue> {
        // TODO: These should come from the state variables \thinmuskip,
        // \mediummuskip, and \thickmuskip.
        // TODO: These should be MuGlue, not plain Glue
        let thinskip = Glue {
            space: Dimen::from_unit(3.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::zero()),
            shrink: SpringDimen::Dimen(Dimen::zero()),
        };
        let mediumskip = Glue {
            space: Dimen::from_unit(4.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::from_unit(2.0, Unit::Point)),
            shrink: SpringDimen::Dimen(Dimen::from_unit(4.0, Unit::Point)),
        };
        let thickskip = Glue {
            space: Dimen::from_unit(5.0, Unit::Point),
            stretch: SpringDimen::Dimen(Dimen::from_unit(5.0, Unit::Point)),
            shrink: SpringDimen::Dimen(Dimen::zero()),
        };

        if let Some(space) = INTER_ATOM_SPACING.get(&(*left_type, *right_type))
        {
            match (space, style.is_script()) {
                (InterAtomSpacing::None, _) => None,
                (InterAtomSpacing::ThinSkip, _) => Some(thinskip),
                (InterAtomSpacing::ThinSkipNonScript, false) => Some(thinskip),
                (InterAtomSpacing::ThinSkipNonScript, true) => None,
                (InterAtomSpacing::MediumSkipNonScript, false) => {
                    Some(mediumskip)
                }
                (InterAtomSpacing::MediumSkipNonScript, true) => None,
                (InterAtomSpacing::ThickSkipNonScript, false) => {
                    Some(thickskip)
                }
                (InterAtomSpacing::ThickSkipNonScript, true) => None,
            }
        } else {
            panic!("Invalid atom type pair: {:?}/{:?}", left_type, right_type);
        }
    }

    pub fn rebox_box_to_width(
        &mut self,
        tex_box: TeXBox,
        width: Dimen,
    ) -> TeXBox {
        if *tex_box.width() == width {
            return tex_box;
        }

        let mut inner_elems = match tex_box {
            TeXBox::VerticalBox(vbox) => vec![HorizontalListElem::Box {
                tex_box: TeXBox::VerticalBox(vbox),
                shift: Dimen::zero(),
            }],
            TeXBox::HorizontalBox(hbox) => hbox.list,
        };

        let hfil = Glue {
            space: Dimen::zero(),
            stretch: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.0)),
            shrink: SpringDimen::FilDimen(FilDimen::new(FilKind::Fil, 1.0)),
        };

        inner_elems.insert(0, HorizontalListElem::HSkip(hfil.clone()));
        inner_elems.push(HorizontalListElem::HSkip(hfil));

        let hbox = self
            .combine_horizontal_list_into_horizontal_box_with_layout(
                inner_elems,
                &BoxLayout::Fixed(width),
            );

        TeXBox::HorizontalBox(hbox)
    }

    fn convert_math_field_to_box(
        &mut self,
        field: MathField,
        style: &MathStyle,
    ) -> TeXBox {
        match field {
            MathField::Symbol(symbol) => {
                let font = MATH_FONTS
                    .get(&(
                        get_font_style_for_math_style(style),
                        symbol.family_number,
                    ))
                    .unwrap();

                let char_elem = HorizontalListElem::Char {
                    chr: symbol.position_number as char,
                    font: font.clone(),
                };

                let hbox = self.add_to_natural_layout_horizontal_box(
                    HorizontalBox::empty(),
                    char_elem,
                );

                TeXBox::HorizontalBox(hbox)
            }
            MathField::TeXBox(tex_box) => tex_box,
            MathField::MathList(list) => {
                let hlist = self
                    .convert_math_list_to_horizontal_list(list, style.clone());
                let hbox = self
                    .combine_horizontal_list_into_horizontal_box_with_layout(
                        hlist,
                        &BoxLayout::Natural,
                    );

                TeXBox::HorizontalBox(hbox)
            }
        }
    }

    fn translate_op_atom_nucleus(
        &mut self,
        nucleus: Option<MathField>,
        current_style: &MathStyle,
    ) -> TranslatedNucleus {
        match nucleus {
            Some(MathField::Symbol(symbol)) => {
                let font = &MATH_FONTS[&(
                    get_font_style_for_math_style(current_style),
                    symbol.family_number,
                )];

                let position_number = match current_style {
                    // In DisplayStyle, we fetch the successor
                    // for symbol op atoms
                    MathStyle::DisplayStyle | MathStyle::DisplayStylePrime => {
                        self.state
                            .with_metrics_for_font(font, |metrics| {
                                metrics.get_successor(
                                    symbol.position_number as char,
                                )
                            })
                            .unwrap() as u8
                    }
                    _ => symbol.position_number,
                };

                let elem = HorizontalListElem::Char {
                    chr: position_number as char,
                    font: font.clone(),
                };

                let boxed_elem = self.add_to_natural_layout_horizontal_box(
                    HorizontalBox::empty(),
                    elem,
                );

                let sym_font = &MATH_FONTS
                    [&(get_font_style_for_math_style(&current_style), 2)];
                let axis_height = self
                    .state
                    .with_metrics_for_font(sym_font, |metrics| {
                        metrics.get_font_dimension(22)
                    })
                    .unwrap();

                let shift =
                    axis_height - (boxed_elem.height - boxed_elem.depth) / 2;

                let char_elem = HorizontalListElem::Box {
                    tex_box: TeXBox::HorizontalBox(boxed_elem),
                    shift,
                };

                TranslatedNucleus {
                    translation: vec![char_elem],
                    nucleus_is_symbol: true,
                    effective_height: Dimen::zero(),
                    effective_depth: Dimen::zero(),
                }
            }
            Some(field) => {
                let nucleus_box =
                    self.convert_math_field_to_box(field, &current_style);

                let height = *nucleus_box.height();
                let depth = *nucleus_box.depth();

                TranslatedNucleus {
                    translation: vec![HorizontalListElem::Box {
                        tex_box: nucleus_box,
                        shift: Dimen::zero(),
                    }],
                    nucleus_is_symbol: false,
                    effective_height: height,
                    effective_depth: depth,
                }
            }
            None => TranslatedNucleus {
                translation: vec![],
                nucleus_is_symbol: false,
                effective_height: Dimen::zero(),
                effective_depth: Dimen::zero(),
            },
        }
    }

    fn translate_atom_nucleus(
        &mut self,
        nucleus: Option<MathField>,
        current_style: &MathStyle,
    ) -> TranslatedNucleus {
        match nucleus {
            Some(MathField::Symbol(symbol)) => {
                let font = &MATH_FONTS[&(
                    get_font_style_for_math_style(current_style),
                    symbol.family_number,
                )];

                let char_elem = HorizontalListElem::Char {
                    chr: symbol.position_number as char,
                    font: font.clone(),
                };

                TranslatedNucleus {
                    translation: vec![char_elem],
                    nucleus_is_symbol: true,
                    effective_height: Dimen::zero(),
                    effective_depth: Dimen::zero(),
                }
            }
            Some(field) => {
                let nucleus_box =
                    self.convert_math_field_to_box(field, &current_style);

                let height = *nucleus_box.height();
                let depth = *nucleus_box.depth();

                TranslatedNucleus {
                    translation: vec![HorizontalListElem::Box {
                        tex_box: nucleus_box,
                        shift: Dimen::zero(),
                    }],
                    nucleus_is_symbol: false,
                    effective_height: height,
                    effective_depth: depth,
                }
            }
            None => TranslatedNucleus {
                translation: vec![],
                nucleus_is_symbol: false,
                effective_height: Dimen::zero(),
                effective_depth: Dimen::zero(),
            },
        }
    }

    fn add_superscripts_and_subscripts_to_atom_with_translated_nucleus(
        &mut self,
        superscript: Option<MathField>,
        subscript: Option<MathField>,
        translated_nucleus: TranslatedNucleus,
        current_style: &MathStyle,
    ) -> Vec<HorizontalListElem> {
        let sup_sym_font = &MATH_FONTS
            [&(get_font_style_for_math_style(&current_style.up_arrow()), 2)];
        let sub_sym_font = &MATH_FONTS[&(
            get_font_style_for_math_style(&current_style.down_arrow()),
            2,
        )];

        let sup_drop = self
            .state
            .with_metrics_for_font(sup_sym_font, |metrics| {
                metrics.get_font_dimension(18)
            })
            .unwrap();
        let sub_drop = self
            .state
            .with_metrics_for_font(sub_sym_font, |metrics| {
                metrics.get_font_dimension(19)
            })
            .unwrap();

        let sym_font =
            &MATH_FONTS[&(get_font_style_for_math_style(&current_style), 2)];

        // The amount that the superscript and subscript will be
        // shifted with respect to the nucleus. Called u and v in
        // the TeXbook.
        let mut sup_shift = if translated_nucleus.nucleus_is_symbol {
            Dimen::zero()
        } else {
            translated_nucleus.effective_height - sup_drop
        };
        let mut sub_shift = if translated_nucleus.nucleus_is_symbol {
            Dimen::zero()
        } else {
            translated_nucleus.effective_depth + sub_drop
        };

        // TODO(xymostech): Pull this from \scriptspace
        let scriptspace = Dimen::from_unit(0.5, Unit::Point);

        let sub_sup_translation = match (superscript, subscript) {
            (Some(superscript), None) => {
                let mut sup_box = self.convert_math_field_to_box(
                    superscript,
                    &current_style.up_arrow(),
                );
                *sup_box.mut_width() = *sup_box.width() + scriptspace;

                let (sup_shift_for_style, x_height) = self
                    .state
                    .with_metrics_for_font(sym_font, |metrics| {
                        let sup_shift_for_style = match current_style {
                            MathStyle::DisplayStyle => {
                                metrics.get_font_dimension(13)
                            }
                            MathStyle::DisplayStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::TextStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::ScriptStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::ScriptScriptStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            _ => metrics.get_font_dimension(14),
                        };

                        (sup_shift_for_style, metrics.get_font_dimension(5))
                    })
                    .unwrap();

                sup_shift = max(
                    max(sup_shift, sup_shift_for_style),
                    *sup_box.depth() + x_height.abs() / 4,
                );

                Some(HorizontalListElem::Box {
                    tex_box: sup_box,
                    shift: sup_shift,
                })
            }
            (None, Some(subscript)) => {
                let mut sub_box = self.convert_math_field_to_box(
                    subscript,
                    &current_style.down_arrow(),
                );
                *sub_box.mut_width() = *sub_box.width() + scriptspace;

                let (sub1, x_height) = self
                    .state
                    .with_metrics_for_font(sym_font, |metrics| {
                        (
                            metrics.get_font_dimension(16),
                            metrics.get_font_dimension(5),
                        )
                    })
                    .unwrap();

                sub_shift = max(
                    max(sub_shift, sub1),
                    *sub_box.height() - x_height.abs() * 4 / 5,
                );

                Some(HorizontalListElem::Box {
                    tex_box: sub_box,
                    shift: sub_shift * -1,
                })
            }
            (Some(superscript), Some(subscript)) => {
                let mut sup_box = self.convert_math_field_to_box(
                    superscript,
                    &current_style.up_arrow(),
                );
                *sup_box.mut_width() = *sup_box.width() + scriptspace;

                let (sup_shift_for_style, sub_2, x_height) = self
                    .state
                    .with_metrics_for_font(sym_font, |metrics| {
                        let sup_shift_for_style = match current_style {
                            MathStyle::DisplayStyle => {
                                metrics.get_font_dimension(13)
                            }
                            MathStyle::DisplayStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::TextStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::ScriptStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            MathStyle::ScriptScriptStylePrime => {
                                metrics.get_font_dimension(15)
                            }
                            _ => metrics.get_font_dimension(14),
                        };

                        (
                            sup_shift_for_style,
                            metrics.get_font_dimension(17),
                            metrics.get_font_dimension(5),
                        )
                    })
                    .unwrap();

                sup_shift = max(
                    max(sup_shift, sup_shift_for_style),
                    *sup_box.depth() + x_height.abs() / 4,
                );

                let mut sub_box = self.convert_math_field_to_box(
                    subscript,
                    &current_style.down_arrow(),
                );
                *sub_box.mut_width() = *sub_box.width() + scriptspace;

                let sup_height = *sup_box.height();
                let sup_depth = *sup_box.depth();
                let sub_height = *sub_box.height();
                let sub_depth = *sub_box.depth();

                sub_shift = max(sub_shift, sub_2);

                let ext_font = &MATH_FONTS
                    [&(get_font_style_for_math_style(&current_style), 3)];
                let default_rule_thickness = self
                    .state
                    .with_metrics_for_font(ext_font, |metrics| {
                        metrics.get_font_dimension(8)
                    })
                    .unwrap();

                if (sup_shift - sup_depth) - (sub_height - sub_shift)
                    < default_rule_thickness * 4
                {
                    sub_shift = default_rule_thickness * 4 + sub_height
                        - (sup_shift - sup_depth);
                    assert!(
                        (sup_shift - sup_depth) - (sub_height - sub_shift)
                            == default_rule_thickness * 4
                    );

                    let final_shift =
                        x_height.abs() * 4 / 5 - (sup_shift - sup_depth);

                    if final_shift > Dimen::zero() {
                        sup_shift = sup_shift + final_shift;
                        sub_shift = sub_shift - final_shift;
                    }
                }

                let skip_dist = sup_shift + sub_shift - sup_depth - sub_height;
                assert!(
                    sup_height + sup_depth + skip_dist + sub_height + sub_depth
                        == sup_height + sup_shift + sub_depth + sub_shift
                );

                let max_width = max(*sup_box.width(), *sub_box.width());

                let supsub_box = VerticalBox {
                    // NOTE: The TeXbook says that the height of
                    // this resulting box should be sup_height +
                    // sup_shift and the depth should be sub_shift
                    // + sub_depth. However, experimentally I've
                    // found that the boxes actually produced have
                    // the sub_shift included in the height. This
                    // necessitates that the box be shifted down by
                    // sub_shift to put the baseline in the correct
                    // spot, which is implemented below.
                    height: sup_height + sup_shift + sub_shift,
                    depth: sub_depth,
                    width: max_width,

                    list: vec![
                        VerticalListElem::Box {
                            tex_box: sup_box,
                            shift: Dimen::zero(),
                        },
                        VerticalListElem::VSkip(Glue::from_dimen(skip_dist)),
                        VerticalListElem::Box {
                            tex_box: sub_box,
                            shift: Dimen::zero(),
                        },
                    ],
                    glue_set_ratio: None,
                };

                Some(HorizontalListElem::Box {
                    tex_box: TeXBox::VerticalBox(supsub_box),
                    shift: sub_shift * -1,
                })
            }
            (None, None) => None,
        };

        let mut translation = translated_nucleus.translation;
        if let Some(list_elem) = sub_sup_translation {
            translation.push(list_elem);
        }

        translation
    }

    fn generate_delimiter_box(
        &mut self,
        maybe_delim: Option<MathDelimiter>,
        _min_size: Dimen,
    ) -> TeXBox {
        match maybe_delim {
            None => {
                let mut empty_hbox = HorizontalBox::empty();
                // TODO(xymostech): This should come from \nulldelimiterspace
                empty_hbox.width = Dimen::from_unit(1.2, Unit::Point);
                TeXBox::HorizontalBox(empty_hbox)
            }
            Some(_delim) => {
                panic!("unimplemented");
            }
        }
    }

    pub fn convert_math_list_to_horizontal_list(
        &mut self,
        list: MathList,
        start_style: MathStyle,
    ) -> Vec<HorizontalListElem> {
        let mut elems_after_first_pass: Vec<TranslatedMathListElem> =
            Vec::new();
        let mut current_style = start_style.clone();
        let mut prev_atom_kind = None;

        for elem in list {
            match elem {
                MathListElem::Atom(atom) => {
                    let atom_kind = match atom.kind {
                        AtomKind::Ord
                        | AtomKind::Open
                        | AtomKind::Inner
                        | AtomKind::Op => atom.kind,
                        AtomKind::Bin => match prev_atom_kind {
                            None => AtomKind::Ord,
                            Some(AtomKind::Bin) => AtomKind::Ord,
                            Some(AtomKind::Op) => AtomKind::Ord,
                            Some(AtomKind::Rel) => AtomKind::Ord,
                            Some(AtomKind::Open) => AtomKind::Ord,
                            Some(AtomKind::Punct) => AtomKind::Ord,
                            _ => AtomKind::Bin,
                        },
                        AtomKind::Rel | AtomKind::Close | AtomKind::Punct => {
                            if prev_atom_kind == Some(AtomKind::Bin) {
                                let last_atom = elems_after_first_pass
                                    .iter_mut()
                                    .rev()
                                    .find(|item| match item {
                                        TranslatedMathListElem::Atom(_) => true,
                                        _ => false,
                                    })
                                    .unwrap();

                                match last_atom {
                                    TranslatedMathListElem::Atom(atom) => {
                                        assert!(atom.kind == AtomKind::Bin);
                                        atom.kind = AtomKind::Ord;
                                    }
                                    _ => unreachable!(),
                                }
                            }

                            atom.kind
                        }
                        k => panic!("Unimplemented atom kind: {:?}", k),
                    };

                    prev_atom_kind = Some(atom_kind);

                    let translated_nucleus = if atom.kind == AtomKind::Op {
                        self.translate_op_atom_nucleus(
                            atom.nucleus,
                            &current_style,
                        )
                    } else {
                        self.translate_atom_nucleus(
                            atom.nucleus,
                            &current_style,
                        )
                    };

                    let atom_translation = self.add_superscripts_and_subscripts_to_atom_with_translated_nucleus(atom.superscript, atom.subscript, translated_nucleus, &current_style);

                    let translated_atom = TranslatedMathAtom {
                        kind: atom_kind,
                        translation: atom_translation,
                    };

                    elems_after_first_pass
                        .push(TranslatedMathListElem::Atom(translated_atom));
                }
                MathListElem::GeneralizedFraction(GeneralizedFraction {
                    left_delim,
                    right_delim,
                    bar_height,
                    numerator,
                    denominator,
                }) => {
                    let numerator_style = match &current_style {
                        MathStyle::DisplayStyle => MathStyle::TextStyle,
                        MathStyle::DisplayStylePrime => {
                            MathStyle::TextStylePrime
                        }
                        other_style => other_style.up_arrow(),
                    };

                    let denominator_style = match &current_style {
                        MathStyle::DisplayStyle => MathStyle::TextStylePrime,
                        MathStyle::DisplayStylePrime => {
                            MathStyle::TextStylePrime
                        }
                        other_style => other_style.down_arrow(),
                    };

                    let translated_numerator = self
                        .convert_math_list_to_horizontal_list(
                            numerator,
                            numerator_style,
                        );
                    let translated_denominator = self
                        .convert_math_list_to_horizontal_list(
                            denominator,
                            denominator_style,
                        );

                    let mut numerator_box = TeXBox::HorizontalBox(self.combine_horizontal_list_into_horizontal_box_with_layout(translated_numerator, &BoxLayout::Natural));
                    let mut denominator_box = TeXBox::HorizontalBox(self.combine_horizontal_list_into_horizontal_box_with_layout(translated_denominator, &BoxLayout::Natural));

                    match numerator_box.width().cmp(denominator_box.width()) {
                        Ordering::Greater => {
                            denominator_box = self.rebox_box_to_width(
                                denominator_box,
                                *numerator_box.width(),
                            );
                        }
                        Ordering::Less => {
                            numerator_box = self.rebox_box_to_width(
                                numerator_box,
                                *denominator_box.width(),
                            );
                        }
                        _ => {}
                    }

                    let sym_font = &MATH_FONTS
                        [&(get_font_style_for_math_style(&current_style), 2)];

                    let (mut numerator_shift, mut denominator_shift) = self
                        .state
                        .with_metrics_for_font(sym_font, |metrics| {
                            if current_style > MathStyle::TextStyle {
                                (
                                    metrics.get_font_dimension(8),
                                    metrics.get_font_dimension(11),
                                )
                            } else if bar_height == Dimen::zero() {
                                (
                                    metrics.get_font_dimension(10),
                                    metrics.get_font_dimension(12),
                                )
                            } else {
                                (
                                    metrics.get_font_dimension(9),
                                    metrics.get_font_dimension(12),
                                )
                            }
                        })
                        .unwrap();

                    let ex_font = &MATH_FONTS
                        [&(get_font_style_for_math_style(&current_style), 3)];

                    let stack = if bar_height == Dimen::zero() {
                        let default_rule_thickness = self
                            .state
                            .with_metrics_for_font(ex_font, |metrics| {
                                metrics.get_font_dimension(8)
                            })
                            .unwrap();

                        let minimum_clearance =
                            if current_style > MathStyle::TextStyle {
                                default_rule_thickness * 7
                            } else {
                                default_rule_thickness * 3
                            };

                        let actual_clearance = (numerator_shift
                            - *numerator_box.depth())
                            - (*denominator_box.height() - denominator_shift);

                        if actual_clearance < minimum_clearance {
                            let extra_clearance =
                                (minimum_clearance - actual_clearance) / 2;
                            numerator_shift = numerator_shift + extra_clearance;
                            denominator_shift =
                                denominator_shift + extra_clearance;
                        }

                        let kern_size = numerator_shift + denominator_shift
                            - *numerator_box.depth()
                            - *denominator_box.height();

                        assert!(
                            *numerator_box.height()
                                + *numerator_box.depth()
                                + kern_size
                                + *denominator_box.height()
                                + *denominator_box.depth()
                                == *numerator_box.height()
                                    + numerator_shift
                                    + denominator_shift
                                    + *denominator_box.depth()
                        );

                        let stack = VerticalBox {
                            height: *numerator_box.height() + numerator_shift,
                            depth: *denominator_box.depth() + denominator_shift,
                            width: *numerator_box.width(),

                            list: vec![
                                VerticalListElem::Box {
                                    tex_box: numerator_box,
                                    shift: Dimen::zero(),
                                },
                                VerticalListElem::VSkip(Glue::from_dimen(
                                    kern_size,
                                )),
                                VerticalListElem::Box {
                                    tex_box: denominator_box,
                                    shift: Dimen::zero(),
                                },
                            ],
                            glue_set_ratio: None,
                        };
                        HorizontalListElem::Box {
                            tex_box: TeXBox::VerticalBox(stack),
                            shift: Dimen::zero(),
                        }
                    } else {
                        panic!("unimplemented");
                    };

                    let min_delim_size = self
                        .state
                        .with_metrics_for_font(sym_font, |metrics| {
                            if current_style > MathStyle::TextStyle {
                                metrics.get_font_dimension(20)
                            } else {
                                metrics.get_font_dimension(21)
                            }
                        })
                        .unwrap();

                    let left_delim_box =
                        self.generate_delimiter_box(left_delim, min_delim_size);
                    let right_delim_box = self
                        .generate_delimiter_box(right_delim, min_delim_size);

                    let axis_height = self
                        .state
                        .with_metrics_for_font(sym_font, |metrics| {
                            metrics.get_font_dimension(22)
                        })
                        .unwrap();

                    let left_shift = axis_height
                        - (*left_delim_box.height() - *left_delim_box.depth())
                            / 2;
                    let right_shift = axis_height
                        - (*right_delim_box.height()
                            - *right_delim_box.depth())
                            / 2;

                    let translated_atom = TranslatedMathAtom {
                        kind: AtomKind::Ord,
                        translation: vec![
                            HorizontalListElem::Box {
                                tex_box: left_delim_box,
                                shift: left_shift,
                            },
                            stack,
                            HorizontalListElem::Box {
                                tex_box: right_delim_box,
                                shift: right_shift,
                            },
                        ],
                    };

                    elems_after_first_pass
                        .push(TranslatedMathListElem::Atom(translated_atom));
                }
                MathListElem::StyleChange(new_style) => {
                    current_style = new_style.clone();
                    elems_after_first_pass
                        .push(TranslatedMathListElem::StyleChange(new_style));
                }
                _ => {
                    panic!("unimplemented math list elem: {:?}", elem);
                }
            }
        }

        let mut resulting_horizontal_list: Vec<HorizontalListElem> = Vec::new();
        let mut maybe_last_atom_kind: Option<AtomKind> = None;
        let mut current_style = start_style;

        for elem in elems_after_first_pass {
            match elem {
                TranslatedMathListElem::Atom(atom) => {
                    if let Some(last_atom_kind) = maybe_last_atom_kind {
                        if let Some(skip) = self.get_skip_for_atom_pair(
                            &last_atom_kind,
                            &atom.kind,
                            &current_style,
                        ) {
                            resulting_horizontal_list
                                .push(HorizontalListElem::HSkip(skip));
                        }
                    }

                    resulting_horizontal_list.extend(atom.translation);

                    maybe_last_atom_kind = Some(atom.kind);
                }
                TranslatedMathListElem::StyleChange(new_style) => {
                    current_style = new_style;
                }
            }
        }

        resulting_horizontal_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{GlueSetRatio, GlueSetRatioKind};
    use crate::testing::with_parser;

    fn assert_math_list_converts_to_horizontal_list(
        math_list_lines: &[&str],
        horizontal_list_lines: &[&str],
    ) {
        with_parser(math_list_lines, |math_parser| {
            with_parser(horizontal_list_lines, |hlist_parser| {
                let math_list = math_parser.parse_math_list();
                let horizontal_list =
                    hlist_parser.parse_horizontal_list(false, false);

                assert_eq!(
                    math_parser.convert_math_list_to_horizontal_list(
                        math_list,
                        MathStyle::TextStyle
                    ),
                    horizontal_list
                );
            });
        });
    }

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
            parser.parse_assignment(None);

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
    fn it_parses_style_changes() {
        with_parser(
            &[r"\displaystyle \textstyle \scriptstyle \scriptscriptstyle%"],
            |parser| {
                assert_eq!(
                    parser.parse_math_list(),
                    vec![
                        MathListElem::StyleChange(MathStyle::DisplayStyle),
                        MathListElem::StyleChange(MathStyle::TextStyle),
                        MathListElem::StyleChange(MathStyle::ScriptStyle),
                        MathListElem::StyleChange(MathStyle::ScriptScriptStyle),
                    ]
                );
            },
        );
    }

    #[test]
    fn it_parses_superscripts_after_non_atoms() {
        let a_code = MathCode::from_number(0x7161);

        with_parser(&[r"\displaystyle ^a%"], |parser| {
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
    fn it_ends_on_math_shifts() {
        let a_code = MathCode::from_number(0x7161);

        with_parser(&[r"a$%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::Atom(MathAtom::from_math_code(&a_code)),]
            );

            assert_eq!(
                parser.lex_expanded_token(),
                Some(Token::Char('$', Category::MathShift))
            );
        });
    }

    #[test]
    fn it_produces_empty_horizontal_lists_from_empty_math_lists() {
        assert_math_list_converts_to_horizontal_list(&[r"%"], &[r"%"]);
    }

    #[test]
    fn it_produces_single_characters_from_single_atom_math_lists() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a%"],
            &[r"\font\teni=cmmi10\teni a%"],
        );
    }

    #[test]
    fn it_produces_multiple_characters_from_multiple_ord_math_lists() {
        assert_math_list_converts_to_horizontal_list(
            &[r"ab%"],
            &[r"\font\teni=cmmi10\teni ab%"],
        );
    }

    #[test]
    fn it_adds_space_between_atoms_of_different_types_in_math_lists() {
        // o = ord
        // p = op
        // b = bin
        // r = rel
        // n = open
        // c = close
        // t = punct
        assert_math_list_converts_to_horizontal_list(
            &[
                r#"\mathcode`o="006F%"#,
                r#"\mathcode`p="1070%"#,
                r#"\mathcode`b="2062%"#,
                r#"\mathcode`r="3072%"#,
                r#"\mathcode`n="406E%"#,
                r#"\mathcode`c="5063%"#,
                r#"\mathcode`t="6074%"#,
                r"oopoboronocoto%",
                r"pprpnpcptpo%",
                r"bnobo%",
                r"rrnrcrtr%",
                r"nncntn%",
                r"cctc%",
                r"tt%",
            ],
            &[
                r"\def\,{\hskip 3pt}%",
                r"\def\>{\hskip 4pt plus 2pt minus 4pt}%",
                r"\def\;{\hskip 5pt plus 5pt}%",
                r"\def\p{\raise 86472sp \hbox{p}}%",
                r"oo\,\p\,o\>b\>o\;r\;onocot\,o\,%",
                r"\p\,\p\;r\;\p n\p c\,\p t\,\p\,o\>%",
                r"b\>no\>b\>o\;%",
                r"rr\;nrc\;rt\,r\;%",
                r"nncnt\,n%",
                r"cct\,c%",
                r"t\,t%",
            ],
        );
    }

    #[test]
    fn it_does_not_add_some_inter_atom_space_in_script_styles() {
        // o = ord
        // b = bin
        // r = rel
        // p = punct
        assert_math_list_converts_to_horizontal_list(
            &[
                r#"\mathcode`o="006F%"#,
                r#"\mathcode`b="2062%"#,
                r#"\mathcode`r="3072%"#,
                r#"\mathcode`p="6070%"#,
                r"\displaystyle orpob%",
                r"\textstyle orpob%",
                r"\scriptstyle orpob%",
                r"\scriptscriptstyle orpob%",
            ],
            &[
                r"\font\sevenrm=cmr7%",
                r"\font\fiverm=cmr5%",
                r"\def\,{\hskip 3pt}%",
                r"\def\>{\hskip 4pt plus 2pt minus 4pt}%",
                r"\def\;{\hskip 5pt plus 5pt}%",
                r"o\;rp\,o\>b\>%",
                r"o\;rp\,o\>b%",
                r"\sevenrm%",
                r"orpob%",
                r"\fiverm%",
                r"orpob%",
            ],
        );
    }

    #[test]
    fn it_chooses_correct_fonts_for_different_styles() {
        assert_math_list_converts_to_horizontal_list(
            &[
                r"\displaystyle a%",
                r"\textstyle a%",
                r"\scriptstyle a%",
                r"\scriptscriptstyle a%",
            ],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\teni a%",
                r"\teni a%",
                r"\seveni a%",
                r"\fivei a%",
            ],
        );
    }

    #[test]
    fn it_parses_math_fields_as_elements() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);
        let c_code = MathCode::from_number(0x7163);
        let d_code = MathCode::from_number(0x7164);

        with_parser(&[r"a{bc}d%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                    MathListElem::Atom(MathAtom::from_math_list(vec![
                        MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                        MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    ],)),
                    MathListElem::Atom(MathAtom::from_math_code(&d_code)),
                ]
            );
        });
    }

    #[test]
    fn it_pushes_state_in_math_fields() {
        let one_code = MathCode::from_number(0x7031);
        let two_code = MathCode::from_number(0x7032);

        with_parser(
            &[
                r"\count0=1 \number\count0%",
                r"{\number\count0 \count0=2 \number\count0}%",
                r"\number\count0%",
            ],
            |parser| {
                assert_eq!(
                    parser.parse_math_list(),
                    vec![
                        MathListElem::Atom(MathAtom::from_math_code(&one_code)),
                        MathListElem::Atom(MathAtom::from_math_list(vec![
                            MathListElem::Atom(MathAtom::from_math_code(
                                &one_code
                            )),
                            MathListElem::Atom(MathAtom::from_math_code(
                                &two_code
                            )),
                        ],)),
                        MathListElem::Atom(MathAtom::from_math_code(&one_code)),
                    ]
                );
            },
        );
    }

    #[test]
    fn it_converts_math_field_nuclei_to_boxes() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a{bc}d%"],
            &[r"\font\teni=cmmi10\teni%", r"a\hbox{bc}d%"],
        );
    }

    #[test]
    fn it_ignores_spaces_in_math_lists() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);
        let c_code = MathCode::from_number(0x7163);
        let d_code = MathCode::from_number(0x7164);

        with_parser(&[r"a b ", r"c          d"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![
                    MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    MathListElem::Atom(MathAtom::from_math_code(&d_code)),
                ]
            )
        });
    }

    #[test]
    fn it_converts_superscripts_to_raised_boxes() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a^b%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\setbox0=\hbox{\seveni b}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\teni a\raise 237825sp \box0%",
            ],
        );

        assert_math_list_converts_to_horizontal_list(
            &[r"a^{b^c}%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\setbox0=\hbox{\fivei c}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\setbox1=\hbox{\seveni b\raise 197774sp \box0}%",
                r"\count1=\wd1%",
                r"\advance\count1 by 32768%",
                r"\wd1=\count1 sp%",
                r"\teni a\raise 237825sp \box1%",
            ],
        );
    }

    #[test]
    fn boxes_can_be_used_as_nuclei() {
        assert_math_list_converts_to_horizontal_list(
            &[r"\hbox{ab}^{\hbox{c}}%"],
            &[
                r"\setbox0=\hbox{\hbox{c}}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\hbox{ab}\raise 293093sp \box0%",
            ],
        );
    }

    #[test]
    fn it_converts_subscripts_to_lowered_boxes() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a_b%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\setbox0=\hbox{\seveni b}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\teni a\lower 98303sp \box0%",
            ],
        );

        assert_math_list_converts_to_horizontal_list(
            &[r"a_{b_c}%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\setbox0=\hbox{\fivei c}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\setbox1=\hbox{\seveni b\lower 65536sp \box0}%",
                r"\count1=\wd1%",
                r"\advance\count1 by 32768%",
                r"\wd1=\count1 sp%",
                r"\teni a\lower 98303sp \box1%",
            ],
        );
    }

    #[test]
    fn it_converts_subscripts_in_superscripts() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a^{b_c}%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\setbox0=\hbox{\fivei c}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\setbox1=\hbox{\seveni b\lower 65536sp \box0}%",
                r"\count1=\wd1%",
                r"\advance\count1 by 32768%",
                r"\wd1=\count1 sp%",
                r"\teni a\raise 237825sp \box1%",
            ],
        );
    }

    #[test]
    fn it_converts_superscripts_in_subscripts() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a_{b^c}%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\setbox0=\hbox{\fivei c}%",
                r"\count0=\wd0%",
                r"\advance\count0 by 32768%",
                r"\wd0=\count0 sp%",
                r"\setbox1=\hbox{\seveni b\raise 131071sp \box0}%",
                r"\count1=\wd1%",
                r"\advance\count1 by 32768%",
                r"\wd1=\count1 sp%",
                r"\teni a\lower 98303sp \box1%",
            ],
        );
    }

    #[test]
    fn it_converts_superscript_subscript_pairs() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a^b_c%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\def\addscriptspace#1{%",
                r"  \count0=\wd#1%",
                r"  \advance\count0 by 32768 %",
                r"  \wd#1=\count0 sp}%",
                r"\teni a\lower 162016sp \vbox{%",
                r"  \setbox0=\hbox{\seveni b}%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"  \vskip 202323sp%",
                r"  \nointerlineskip%",
                r"  \setbox0=\hbox{\seveni c}%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"}%",
            ],
        );

        assert_math_list_converts_to_horizontal_list(
            &[r"a^{b^c_c}_{c^b_b}%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\def\addscriptspace#1{%",
                r"  \count0=\wd#1%",
                r"  \advance\count0 by 32768 %",
                r"  \wd#1=\count0 sp}%",
                r"\teni a\lower 264687sp \vbox{%",
                r"  \setbox0=\hbox{%",
                r"    \seveni b%",
                r"    \lower 131071sp \vbox{%",
                r"      \setbox1=\hbox{\fivei c}%",
                r"      \addscriptspace 1%",
                r"      \copy1%",
                r"      \vskip 187761sp%",
                r"      \nointerlineskip%",
                r"      \box1%",
                r"    }%",
                r"  }%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"  \vskip 104852sp%",
                r"  \nointerlineskip%",
                r"  \setbox0=\hbox{%",
                r"    \seveni c%",
                r"    \lower 174393sp \vbox{%",
                r"      \setbox1=\hbox{\fivei b}%",
                r"      \addscriptspace 1%",
                r"      \copy1%",
                r"      \vskip 104852sp%",
                r"      \nointerlineskip%",
                r"      \box1%",
                r"    }%",
                r"  }%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"}%",
            ],
        );
    }

    #[test]
    fn bin_atoms_are_converted_to_ords_in_certain_situations() {
        // o = ord
        // p = op
        // b = bin
        // r = rel
        // n = open
        // c = close
        // t = punct
        assert_math_list_converts_to_horizontal_list(
            &[
                r#"\mathcode`o="006F%"#,
                r#"\mathcode`p="1070%"#,
                r#"\mathcode`b="2062%"#,
                r#"\mathcode`r="3072%"#,
                r#"\mathcode`n="406E%"#,
                r#"\mathcode`c="5063%"#,
                r#"\mathcode`t="6074%"#,
                // Bins at the start of a list are converted to ords
                r"bo%",
                // Bins following bin, op, rel, open, punct are converted to ords
                r"bb o pb o rb o nb o tb o%",
                // Bins before rel, close, punct are converted to ords
                r"br o bc o bt o%",
            ],
            &[
                r"\def\,{\hskip 3pt}%",
                r"\def\>{\hskip 4pt plus 2pt minus 4pt}%",
                r"\def\;{\hskip 5pt plus 5pt}%",
                r"\def\p{\raise 86472sp \hbox{p}}%",
                r"bo%",
                r"\>b\>bo\,\p\,bo\;r\;bonbot\,bo%",
                r"b\;r\;obcobt\,o%",
            ],
        );
    }

    #[test]
    fn it_parses_explicit_char_symbols() {
        assert_math_list_converts_to_horizontal_list(
            &[r#"\mathcode`s="0350%"#, r"\char97 \char122%", r"\char115%"],
            &[
                r"\font\teni=cmmi10%",
                r"\font\tenex=cmex10%",
                r"\teni az%",
                r"\tenex \char80%",
            ],
        );
    }

    #[test]
    fn it_correctly_finds_operator_successors() {
        assert_math_list_converts_to_horizontal_list(
            &[
                // Plain 'a' in cmmi
                r#"\mathcode`a="1161%"#,
                // \sum
                r#"\mathchardef\sum="1350%"#,
                // \int
                r#"\mathchardef\int="1352%"#,
                r"a\sum\int \displaystyle a\sum\int%",
            ],
            &[
                r"\def\,{\hskip 3pt}%",
                r"\font\teni=cmmi10%",
                r"\font\tenex=cmex10%",
                // normal a
                r"\raise 22756sp \hbox{\teni a}\,%",
                // small \sum
                r"\raise 491524sp \hbox{\tenex \char80}\,%",
                // small \int
                r"\raise 527932sp \hbox{\tenex \char82}\,%",
                // normal a
                r"\raise 22756sp \hbox{\teni a}\,%",
                // big \sum
                r"\raise 622596sp \hbox{\tenex \char88}\,%",
                // big \int
                r"\raise 892025sp \hbox{\tenex \char90}%",
            ],
        );
    }

    #[test]
    fn it_raises_op_atoms_to_the_baseline() {
        assert_math_list_converts_to_horizontal_list(
            &[
                // \sum as an ord
                r#"\mathchardef\sumord="0350%"#,
                // \sum as an op
                r#"\mathchardef\sumop="1350%"#,
                r"\sumord \sumop%",
            ],
            &[
                r"\def\,{\hskip 3pt}%",
                r"\font\tenex=cmex10%",
                r"\tenex \char80\,%",
                r"\raise 491524sp \hbox{\tenex \char80}%",
            ],
        );
    }

    #[test]
    fn it_reboxes_boxes_to_widths() {
        with_parser(
            &[
                r"\def\hfil{\hskip 0pt plus 1fil minus 1fil}%",
                r"\def\hfill{\hskip 0pt plus 1fill minus 1fill}%",
                r"\setbox0=\hbox to 5pt{a}%",
                r"\hbox{\copy0}%",
                r"\hbox to 10pt{\copy0\hfil}%",
                r"\hbox{\copy0\hfil}%",
                r"\hbox{\copy0\hfill}%",
                r"\setbox1=\vbox{\noindent a}%",
                r"\wd1=5pt%",
                // result boxes
                r"\hbox to 10pt{\hfil\copy0\hfil}%",
                r"\hbox to 10pt{\hfil\copy0\hfil\hfil}%",
                r"\hbox to 10pt{\hfil\copy0\hfill\hfil}%",
                r"\hbox to 10pt{\hfil\copy1\hfil}%",
            ],
            |parser| {
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                let five_pt_box = parser.parse_box().unwrap();
                let ten_pt_box_with_hfil = parser.parse_box().unwrap();
                let box_with_hfil = parser.parse_box().unwrap();
                let box_with_hfill = parser.parse_box().unwrap();
                parser.parse_assignment(None);
                parser.parse_assignment(None);
                let vbox = parser.state.get_box_copy(1).unwrap();

                let ten_pt = Dimen::from_unit(10.0, Unit::Point);

                // 5pt box is expanded to 10pt
                let five_pt_box_reboxed_to_ten_pt =
                    parser.rebox_box_to_width(five_pt_box, ten_pt);
                assert_eq!(
                    five_pt_box_reboxed_to_ten_pt,
                    parser.parse_box().unwrap()
                );
                assert_eq!(*five_pt_box_reboxed_to_ten_pt.width(), ten_pt);
                assert_eq!(
                    match five_pt_box_reboxed_to_ten_pt {
                        TeXBox::HorizontalBox(hbox) => hbox.glue_set_ratio,
                        _ => panic!("Not a horizontal box!"),
                    },
                    Some(GlueSetRatio::from(
                        GlueSetRatioKind::from_fil_kind(&FilKind::Fil),
                        2.5
                    )),
                );

                // 10pt box isn't changed at all when reboxed to 10pt
                let ten_pt_box_with_hfil_reboxed_to_ten_pt = parser
                    .rebox_box_to_width(ten_pt_box_with_hfil.clone(), ten_pt);
                assert_eq!(*ten_pt_box_with_hfil.width(), ten_pt);
                assert_eq!(
                    ten_pt_box_with_hfil_reboxed_to_ten_pt,
                    ten_pt_box_with_hfil
                );
                assert_eq!(
                    *ten_pt_box_with_hfil_reboxed_to_ten_pt.width(),
                    ten_pt
                );

                // boxes with hfil and hfill in them are expanded to 5pt
                let box_with_hfil_reboxed_to_ten_pt =
                    parser.rebox_box_to_width(box_with_hfil, ten_pt);
                assert_eq!(
                    box_with_hfil_reboxed_to_ten_pt,
                    parser.parse_box().unwrap()
                );
                assert_eq!(*box_with_hfil_reboxed_to_ten_pt.width(), ten_pt);
                assert_eq!(
                    match box_with_hfil_reboxed_to_ten_pt {
                        TeXBox::HorizontalBox(hbox) => hbox.glue_set_ratio,
                        _ => panic!("Not a horizontal box!"),
                    },
                    Some(GlueSetRatio::from(
                        GlueSetRatioKind::from_fil_kind(&FilKind::Fil),
                        5.0 / 3.0
                    )),
                );

                let box_with_hfill_reboxed_to_ten_pt =
                    parser.rebox_box_to_width(box_with_hfill, ten_pt);
                assert_eq!(
                    box_with_hfill_reboxed_to_ten_pt,
                    parser.parse_box().unwrap()
                );
                assert_eq!(*box_with_hfill_reboxed_to_ten_pt.width(), ten_pt);
                assert_eq!(
                    match box_with_hfill_reboxed_to_ten_pt {
                        TeXBox::HorizontalBox(hbox) => hbox.glue_set_ratio,
                        _ => panic!("Not a horizontal box!"),
                    },
                    Some(GlueSetRatio::from(
                        GlueSetRatioKind::from_fil_kind(&FilKind::Fill),
                        5.0
                    )),
                );

                // vboxes aren't unboxed and are just surrounded by \hfil
                let vbox_reboxed_to_ten_pt =
                    parser.rebox_box_to_width(vbox, ten_pt);
                assert_eq!(vbox_reboxed_to_ten_pt, parser.parse_box().unwrap());
                assert_eq!(*vbox_reboxed_to_ten_pt.width(), ten_pt);
                assert_eq!(
                    match vbox_reboxed_to_ten_pt {
                        TeXBox::HorizontalBox(hbox) => hbox.glue_set_ratio,
                        _ => panic!("Not a horizontal box!"),
                    },
                    Some(GlueSetRatio::from(
                        GlueSetRatioKind::from_fil_kind(&FilKind::Fil),
                        2.5
                    )),
                );
            },
        );
    }

    #[test]
    fn it_parses_basic_generalized_fractions() {
        let a_code = MathCode::from_number(0x7161);
        let b_code = MathCode::from_number(0x7162);
        let c_code = MathCode::from_number(0x7163);

        with_parser(&[r"a\atop b%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::GeneralizedFraction(GeneralizedFraction {
                    left_delim: None,
                    right_delim: None,
                    bar_height: Dimen::zero(),
                    numerator: vec![MathListElem::Atom(
                        MathAtom::from_math_code(&a_code)
                    ),],
                    denominator: vec![MathListElem::Atom(
                        MathAtom::from_math_code(&b_code)
                    ),],
                })]
            );
        });

        with_parser(&[r"{a\atop b} \atop c%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::GeneralizedFraction(GeneralizedFraction {
                    left_delim: None,
                    right_delim: None,
                    bar_height: Dimen::zero(),
                    numerator: vec![MathListElem::Atom(
                        MathAtom::from_math_list(vec![
                            MathListElem::GeneralizedFraction(
                                GeneralizedFraction {
                                    left_delim: None,
                                    right_delim: None,
                                    bar_height: Dimen::zero(),
                                    numerator: vec![MathListElem::Atom(
                                        MathAtom::from_math_code(&a_code)
                                    ),],
                                    denominator: vec![MathListElem::Atom(
                                        MathAtom::from_math_code(&b_code)
                                    ),],
                                }
                            )
                        ])
                    ),],
                    denominator: vec![MathListElem::Atom(
                        MathAtom::from_math_code(&c_code)
                    ),],
                })]
            );
        });

        with_parser(&[r"abc \atop abc%"], |parser| {
            assert_eq!(
                parser.parse_math_list(),
                vec![MathListElem::GeneralizedFraction(GeneralizedFraction {
                    left_delim: None,
                    right_delim: None,
                    bar_height: Dimen::zero(),
                    numerator: vec![
                        MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                        MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                        MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    ],
                    denominator: vec![
                        MathListElem::Atom(MathAtom::from_math_code(&a_code)),
                        MathListElem::Atom(MathAtom::from_math_code(&b_code)),
                        MathListElem::Atom(MathAtom::from_math_code(&c_code)),
                    ],
                })]
            );
        });
    }

    #[test]
    #[should_panic(expected = "Ambiguous generalized fraction")]
    fn it_fails_on_ambiguous_generalized_fractions() {
        with_parser(&[r"a \atop b \atop c%"], |parser| {
            parser.parse_math_list();
        });
    }

    #[test]
    fn it_converts_atop_to_centered_vertical_boxes() {
        assert_math_list_converts_to_horizontal_list(
            &[r"a \atop b%"],
            &[
                r"\def\hfil{\hskip 0pt plus 1fil minus 1fil}%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\font\seveni=cmmi7%",
                r"\setbox0=\hbox{}%",
                r"\wd0=1.2pt%",
                r"\setbox1=\hbox{\seveni a}%",
                r"\setbox2=\hbox to\number\wd1 sp{\hfil \seveni b\hfil}%",
                r"\setbox3=\vbox{%",
                r"  \box1%",
                r"  \vskip 198221sp%",
                r"  \nointerlineskip%",
                r"  \box2%",
                r"}%",
                r"\ht3=488321sp%",
                r"\dp3=225995sp%",
                r"\raise2.5pt\copy0%",
                r"\box3%",
                r"\raise2.5pt\copy0%",
            ],
        );

        assert_math_list_converts_to_horizontal_list(
            &[r"{a \atop b} \atop c%"],
            &[
                r"\def\hfil{\hskip 0pt plus 1fil minus 1fil}%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\font\seveni=cmmi7%",
                r"\font\fivei=cmmi5%",
                r"\setbox0=\hbox{}%",
                r"\wd0=1.2pt%",
                r"\setbox1=\hbox{\fivei a}%",
                r"\setbox2=\hbox to\number\wd1 sp{\hfil \fivei b\hfil}%",
                r"\setbox3=\vbox{%",
                r"  \box1%",
                r"  \vskip 146520sp%",
                r"  \nointerlineskip%",
                r"  \box2%",
                r"}%",
                r"\ht3=357250sp%",
                r"\dp3=157909sp%",
                r"\setbox4=\hbox{\hbox{%",
                r"  \raise 1.75pt\copy0%",
                r"  \box3%",
                r"  \raise 1.75pt\copy0%",
                r"}}%",
                r"\setbox5=\hbox to\number\wd4 sp{\hfil \seveni c\hfil}%",
                r"\setbox6=\vbox{%",
                r"  \box4%",
                r"  \vskip 161371sp%",
                r"  \nointerlineskip%",
                r"  \box5%",
                r"}%",
                r"\ht6=648053sp%",
                r"\dp6=225995sp%",
                r"\raise 2.5pt\copy0%",
                r"\box6%",
                r"\raise 2.5pt\copy0%",
            ],
        );
    }

    #[test]
    fn it_adds_correct_space_around_fractions() {
        assert_math_list_converts_to_horizontal_list(
            &[r#"\mathcode`+="202B%"#, r"a + {a \atop b} + c%"],
            &[
                r"\def\hfil{\hskip 0pt plus 1fil minus 1fil}%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\font\tenrm=cmr10%",
                r"\font\teni=cmmi10%",
                r"\font\seveni=cmmi7%",
                r"\setbox0=\hbox{}%",
                r"\wd0=1.2pt%",
                r"\setbox1=\hbox{\seveni a}%",
                r"\setbox2=\hbox to\number\wd1 sp{\hfil \seveni b\hfil}%",
                r"\setbox3=\vbox{%",
                r"  \box1%",
                r"  \vskip 198221sp%",
                r"  \nointerlineskip%",
                r"  \box2%",
                r"}%",
                r"\ht3=488321sp%",
                r"\dp3=225995sp%",
                r"\def\>{\hskip 4pt plus 2pt minus 4pt}%",
                r"\teni a%",
                r#"\>\tenrm \char"2B"#,
                r"\>\hbox{%",
                r"  \raise 2.5pt\copy0%",
                r"  \box3",
                r"  \raise 2.5pt\copy0%",
                r"}%",
                r#"\>\tenrm \char"2B"#,
                r"\>\teni c%",
            ],
        );
    }

    #[test]
    fn it_adds_appropriate_space_between_superscripts_and_subscripts_with_large_nuclei(
    ) {
        assert_math_list_converts_to_horizontal_list(
            &[r"\setbox0=\hbox{}%", r"\ht0=10pt%", r"\box0^a_b%"],
            &[
                r"\font\seveni=cmmi7%",
                r"\def\nointerlineskip{\prevdepth=-1000pt}%",
                r"\def\addscriptspace#1{%",
                r"  \count0=\wd#1%",
                r"  \advance\count0 by 32768 %",
                r"  \wd#1=\count0 sp}%",
                r"\setbox0=\hbox{}%",
                r"\ht0=10pt%",
                r"\box0%",
                r"\lower 162016sp \vbox{%",
                r"  \setbox0=\hbox{\seveni a}%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"  \vskip 336781sp%",
                r"  \nointerlineskip%",
                r"  \setbox0=\hbox{\seveni b}%",
                r"  \addscriptspace 0%",
                r"  \box0%",
                r"}%",
            ],
        );
    }
}

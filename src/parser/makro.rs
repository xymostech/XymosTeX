use crate::category::Category;
use crate::makro::{Macro, MacroListElem};
use crate::parser::Parser;
use crate::token::Token;

fn parse_parameter_number(ch: char) -> usize {
    if ch >= '1' && ch <= '9' {
        ((ch as u8) - ('0' as u8)) as usize
    } else {
        panic!("Invalid number after parameter: {}", ch);
    }
}

impl<'a> Parser<'a> {
    // Parses a parameter list and replacement list into a macro object
    pub fn parse_macro_definition(&mut self) -> Macro {
        let mut parameter_list: Vec<MacroListElem> = Vec::new();

        // When the last character of the parameter list (right before the {)
        // is a #, (e.g. `\def\a#{}`) then the following { is appended to the
        // end of the parameter list and the replacement list. If such a
        // situation occurs, then we set this to the token that we need to add
        // to the end of the replacement list.
        let mut maybe_final_tok: Option<Token> = None;

        loop {
            if let Some(token) = self.lex_unexpanded_token() {
                match token {
                    // We've found the beginning of the replacement list
                    Token::Char(_, Category::BeginGroup) => break,

                    // We've found a parameter token, check the next token
                    Token::Char(_, Category::Parameter) => {
                        match self.lex_unexpanded_token() {
                            // If it's a {, then we're in the special case
                            // mentioned above. Store the token in
                            // `maybe_final_tok` and break out.
                            Some(Token::Char(ch, Category::BeginGroup)) => {
                                parameter_list.push(MacroListElem::Token(Token::Char(
                                    ch,
                                    Category::BeginGroup,
                                )));
                                maybe_final_tok = Some(Token::Char(ch, Category::BeginGroup));
                                break;
                            }
                            // If it's an Other, it's probably a number
                            Some(Token::Char(ch, Category::Other)) => {
                                let index = parse_parameter_number(ch);
                                parameter_list.push(MacroListElem::Parameter(index));
                            }
                            // Anything else is an error
                            _ => panic!("Invalid token found after parameter"),
                        }
                    }

                    // We've found some other kind of token, so simply add it
                    // to the list
                    _ => parameter_list.push(MacroListElem::Token(token)),
                }
            } else {
                panic!("EOF found while parsing macro definition");
            }
        }

        let mut replacement_list: Vec<MacroListElem> = Vec::new();

        // We need to parse a balanced list for the replacement text, so we
        // keep track of the level of grouping
        let mut group_level = 0;

        loop {
            if let Some(token) = self.lex_unexpanded_token() {
                match token {
                    Token::Char(_, Category::EndGroup) => {
                        // If we see a group close and we're at the same group
                        // level as we were at the start, we're done.
                        if group_level == 0 {
                            break;
                        } else {
                            replacement_list.push(MacroListElem::Token(token));
                            group_level -= 1;
                        }
                    }
                    Token::Char(_, Category::BeginGroup) => {
                        replacement_list.push(MacroListElem::Token(token));
                        group_level += 1;
                    }
                    Token::Char(_, Category::Parameter) => {
                        match self.lex_unexpanded_token() {
                            // If we see a parameter token right after another
                            // parameter token, we insert the second token into
                            // our list.
                            Some(Token::Char(ch, Category::Parameter)) => {
                                replacement_list.push(MacroListElem::Token(Token::Char(
                                    ch,
                                    Category::Parameter,
                                )));
                            }
                            Some(Token::Char(ch, Category::Other)) => {
                                let index = parse_parameter_number(ch);
                                replacement_list.push(MacroListElem::Parameter(index));
                            }
                            _ => panic!("Invalid token found after parameter"),
                        }
                    }
                    _ => replacement_list.push(MacroListElem::Token(token)),
                }
            } else {
                panic!("EOF found parsing macro definition");
            }
        }

        // If the special case related to `maybe_final_tok` is happening, we
        // need to add that token to the end of the replacement list.
        if let Some(token) = maybe_final_tok {
            replacement_list.push(MacroListElem::Token(token));
        }

        Macro::new(parameter_list, replacement_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TeXState;

    fn assert_parses_to_macro(lines: &[&str], expected_macro: Macro) {
        let state = TeXState::new();
        let mut parser = Parser::new(lines, &state);

        assert_eq!(
            Some(Token::ControlSequence("def".to_string())),
            parser.lex_unexpanded_token()
        );
        assert_eq!(
            Some(Token::ControlSequence("a".to_string())),
            parser.lex_unexpanded_token()
        );
        assert_eq!(expected_macro, parser.parse_macro_definition());
        assert_eq!(None, parser.lex_unexpanded_token());
    }

    fn try_parsing_macro(lines: &[&str]) {
        let state = TeXState::new();
        let mut parser = Parser::new(lines, &state);

        assert_eq!(
            Some(Token::ControlSequence("def".to_string())),
            parser.lex_unexpanded_token()
        );
        assert_eq!(
            Some(Token::ControlSequence("a".to_string())),
            parser.lex_unexpanded_token()
        );
        parser.parse_macro_definition();
    }

    #[test]
    fn it_parses_empty_macros() {
        assert_parses_to_macro(&["\\def\\a{}%"], Macro::new(Vec::new(), Vec::new()));
    }

    #[test]
    fn it_parses_parameters() {
        assert_parses_to_macro(
            &["\\def\\a#1#2{#1#2}%"],
            Macro::new(
                vec![MacroListElem::Parameter(1), MacroListElem::Parameter(2)],
                vec![MacroListElem::Parameter(1), MacroListElem::Parameter(2)],
            ),
        );
    }

    #[test]
    fn it_parses_tokens() {
        assert_parses_to_macro(
            &["\\def\\a ab{ab}%"],
            Macro::new(
                vec![
                    MacroListElem::Token(Token::Char('a', Category::Letter)),
                    MacroListElem::Token(Token::Char('b', Category::Letter)),
                ],
                vec![
                    MacroListElem::Token(Token::Char('a', Category::Letter)),
                    MacroListElem::Token(Token::Char('b', Category::Letter)),
                ],
            ),
        );
    }

    #[test]
    fn it_handles_double_parameter_tokens() {
        assert_parses_to_macro(
            &["\\def\\a{##}%"],
            Macro::new(
                vec![],
                vec![MacroListElem::Token(Token::Char('#', Category::Parameter))],
            ),
        );
    }

    #[test]
    fn it_handles_final_parameter_special_cases() {
        assert_parses_to_macro(
            &["\\def\\a a#{}%"],
            Macro::new(
                vec![
                    MacroListElem::Token(Token::Char('a', Category::Letter)),
                    MacroListElem::Token(Token::Char('{', Category::BeginGroup)),
                ],
                vec![MacroListElem::Token(Token::Char('{', Category::BeginGroup))],
            ),
        );
    }

    #[test]
    #[should_panic(expected = "EOF found")]
    fn it_fails_on_eof_in_parameter_list() {
        try_parsing_macro(&["\\def\\a a%"]);
    }

    #[test]
    #[should_panic(expected = "EOF found")]
    fn it_fails_on_eof_in_replacement_list() {
        try_parsing_macro(&["\\def\\a a{%"]);
    }

    #[test]
    #[should_panic(expected = "Invalid number after parameter")]
    fn it_fails_on_non_number_in_parameter() {
        try_parsing_macro(&["\\def\\a #.{}%"]);
    }
}

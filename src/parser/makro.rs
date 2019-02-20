use std::collections::HashMap;

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

// For a macro and a parameter at a given index in the parameter list, find the
// index of the next non-token item in the parameter list. (Note that this
// could be at the end of the list, i.e. makro.parameter_list.len(), if all of
// the remaining elements are tokens)
fn get_next_non_token_index(makro: &Macro, parameter_index: usize) -> usize {
    let mut end_index = parameter_index + 1;
    while let Some(MacroListElem::Token(_)) = makro.parameter_list.get(end_index) {
        end_index += 1;
    }
    end_index
}

// Used to keep track of the result of parsing a single token/balanced group
enum SingleTokenGroup {
    BalancedGroup(Token, Vec<Token>, Token),
    SingleToken(Token),
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

    // This parses a list of tokens that is delimited group tokens and has a
    // balanced number of begin and end tokens. It returns the list of tokens
    // and the final ending group token.
    fn parse_balanced_text(&mut self) -> (Vec<Token>, Token) {
        let mut result = Vec::new();
        // Keep track of the number of { and } tokens we've seen, with the
        // grouping increasing for { and decreasing for }.
        let mut group_level = 0;
        loop {
            let token = self.lex_unexpanded_token().unwrap();
            match token {
                Token::Char(_, Category::BeginGroup) => {
                    group_level += 1;
                    result.push(token);
                }
                Token::Char(_, Category::EndGroup) => {
                    if group_level == 0 {
                        // If we see an EndGroup token and we're at the
                        // outermost group , we're done! We return here so we
                        // have access to the final token.
                        return (result, token);
                    } else {
                        group_level -= 1;
                        result.push(token);
                    }
                }
                _ => result.push(token),
            }
        }
    }

    // While we're parsing tokens for macro parameters, we often want to get
    // either a single token or, if the first token is a {, parse an entire
    // balanced group. This function handles that and returns all the
    // information about what was parsed in an enm.
    fn parse_single_token_or_balanced_text(&mut self) -> SingleTokenGroup {
        let token = self.lex_unexpanded_token().unwrap();
        match token {
            Token::Char(_, Category::BeginGroup) => {
                let (inner, close) = self.parse_balanced_text();
                SingleTokenGroup::BalancedGroup(token, inner, close)
            }
            _ => SingleTokenGroup::SingleToken(token),
        }
    }

    // This handles the case we want to parse a single token or balanced group
    // but just want a list of tokens out, where
    //  * with a single token, we just get that token in a list
    //  * with a balanced group, we get all the tokens inside of the {} but not
    //    the {} themselves
    fn parse_single_token_or_balanced_text_unwrapped(&mut self) -> Vec<Token> {
        match self.parse_single_token_or_balanced_text() {
            SingleTokenGroup::SingleToken(token) => vec![token],
            SingleTokenGroup::BalancedGroup(_, inner, _) => inner,
        }
    }

    // Parse 0 or more space tokens and ignore them
    fn parse_optional_spaces(&mut self) {
        while let Some(Token::Char(_, Category::Space)) = self.peek_unexpanded_token() {
            self.lex_unexpanded_token();
        }
    }

    // This handles parsing the tokens for a delimited parameter. The goal is
    // to continue parsing tokens/balanced groups until a sequence of tokens
    // that match the list of delimiters is found, and then returning the
    // tokens that were parsed before then.
    fn parse_delimited_tokens(&mut self, delimiters: &[MacroListElem]) -> Vec<Token> {
        let mut result_tokens: Vec<Token> = Vec::new();

        // When we encounter tokens that match some of the delimiters, we need
        // to hold on to them in case later tokens don't match the following
        // delimiters. In that case, we push these buffered tokens onto the
        // result and start looking at the beginning of the delimiters again.
        // This is where we store those buffered tokens.
        let mut delimiting_tokens_buffer: Vec<Token> = Vec::new();

        // This index keeps track of which delimiter we are currently looking
        // at.
        let mut delimiter_index = 0;

        while delimiter_index < delimiters.len() {
            // Since the slice of delimiters was found by searching a macro's
            // parameter list for tokens, we can be confident that all of the
            // elements will be `MacroListElem::Token`s.
            // TODO(xymostech): Figure out a good way to map `&[MacroListElem]
            // -> &[Token]`
            let expected_token = match &delimiters[delimiter_index] {
                MacroListElem::Token(tok) => tok,
                _ => panic!("Invalid non-token found in delimiter"),
            };

            if let Token::Char(_, Category::BeginGroup) = expected_token {
                // If the token we're looking for is the opening brace of a
                // group, we don't want to parse an entire balanced group, we
                // just want to check if the immediate next token is a {.
                let check_token = self.lex_unexpanded_token().unwrap();

                // TODO(xymostech): This is a duplicate of the if statement
                // down below. Figure out a way to deduplicate this.
                if check_token == *expected_token {
                    delimiter_index += 1;
                    delimiting_tokens_buffer.push(check_token);
                } else {
                    delimiter_index = 0;
                    result_tokens.append(&mut delimiting_tokens_buffer);
                    result_tokens.push(check_token);
                }
            } else {
                match self.parse_single_token_or_balanced_text() {
                    SingleTokenGroup::SingleToken(check_token) => {
                        if check_token == *expected_token {
                            // If we found a single token and it matches the
                            // delimiter, continue looking at the next delimiter,
                            // but store the token we found in our buffer just in
                            // case later tokens don't match.
                            delimiter_index += 1;
                            delimiting_tokens_buffer.push(check_token);
                        } else {
                            // If the single token doesn't match, then we reset
                            // looking at the beginning of the delimiters.
                            delimiter_index = 0;
                            result_tokens.append(&mut delimiting_tokens_buffer);
                            result_tokens.push(check_token);
                        }
                    }
                    SingleTokenGroup::BalancedGroup(open, mut inner, close) => {
                        // If we found a balanced group, this will
                        delimiter_index = 0;
                        result_tokens.append(&mut delimiting_tokens_buffer);
                        result_tokens.push(open);
                        result_tokens.append(&mut inner);
                        result_tokens.push(close);
                    }
                }
            }
        }
        result_tokens
    }

    pub fn parse_replacement_map(&mut self, makro: &Macro) -> HashMap<usize, Vec<Token>> {
        let mut replacement_map: HashMap<usize, Vec<Token>> = HashMap::new();

        // We manually iterate through the replacement_list because when we
        // parse a delimited parameter, we advance through all of the
        // delimiting tokens in a single iteration.
        let mut index = 0;
        while index < makro.parameter_list.len() {
            let elem = &makro.parameter_list[index];

            match elem {
                MacroListElem::Parameter(parameter_index) => {
                    // A parameter is delimited if it's followed immediately by
                    // a token. If the parameter is the last element in the
                    // list, it is not delimited.
                    let is_delimited = match makro.parameter_list.get(index + 1) {
                        Some(MacroListElem::Token(_)) => true,
                        _ => false,
                    };

                    let toks = if is_delimited {
                        let delimiter_last_index = get_next_non_token_index(makro, index);
                        let delimited_toks = self.parse_delimited_tokens(
                            &makro.parameter_list[index + 1..delimiter_last_index],
                        );

                        // The delimiters following the parameter are parsed in
                        // parse_delimited_tokens, so we skip parsing them here.
                        index = delimiter_last_index;

                        delimited_toks
                    } else {
                        index += 1;
                        // When a parameter is undelimited (and only
                        // undelimited!) we skip spaces before parsing the
                        // actual token/balanced group that match the
                        // parameter.
                        self.parse_optional_spaces();
                        self.parse_single_token_or_balanced_text_unwrapped()
                    };
                    replacement_map.insert(*parameter_index, toks);
                }
                MacroListElem::Token(search_token) => {
                    let found_token = self.lex_unexpanded_token().unwrap();
                    if found_token != *search_token {
                        panic!(
                            "Non-matching token found looking for parameter text. \
                             Found {:?}, expected {:?}",
                            found_token, search_token
                        );
                    }
                    index += 1;
                }
            }
        }

        replacement_map
    }
}

#[cfg(test)]
mod tests {
    mod macro_definition {
        use super::super::*;
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

    mod replacement_tokens {
        use super::super::*;

        use crate::state::TeXState;

        fn assert_parses_to_replacements(
            lines: &[&str],
            macro_parameter_list: Vec<MacroListElem>,
            expected_replacements: Vec<(usize, Vec<Token>)>,
        ) {
            let state = TeXState::new();
            let mut parser = Parser::new(lines, &state);

            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::ControlSequence("a".to_string()))
            );

            let makro = Macro::new(macro_parameter_list, Vec::new());
            let expected_replacement_map: HashMap<usize, Vec<Token>> =
                expected_replacements.into_iter().collect();
            let replacement_map = parser.parse_replacement_map(&makro);
            assert_eq!(expected_replacement_map, replacement_map);

            assert_eq!(parser.lex_unexpanded_token(), None);
        }

        fn try_parsing_replacements(lines: &[&str], macro_parameter_list: Vec<MacroListElem>) {
            let state = TeXState::new();
            let mut parser = Parser::new(lines, &state);

            assert_eq!(
                parser.lex_unexpanded_token(),
                Some(Token::ControlSequence("a".to_string()))
            );

            let makro = Macro::new(macro_parameter_list, Vec::new());
            parser.parse_replacement_map(&makro);

            assert_eq!(parser.lex_unexpanded_token(), None);
        }

        #[test]
        fn it_parses_single_tokens_for_parameters() {
            assert_parses_to_replacements(
                &["\\a x%"],
                vec![MacroListElem::Parameter(1)],
                vec![(1, vec![Token::Char('x', Category::Letter)])],
            );
        }

        #[test]
        fn it_parses_balanced_tokens_for_parameters() {
            assert_parses_to_replacements(
                &["\\a {xy}%"],
                vec![MacroListElem::Parameter(1)],
                vec![(
                    1,
                    vec![
                        Token::Char('x', Category::Letter),
                        Token::Char('y', Category::Letter),
                    ],
                )],
            );

            assert_parses_to_replacements(
                &["\\a {w{x{y}}z}%"],
                vec![MacroListElem::Parameter(1)],
                vec![(
                    1,
                    vec![
                        Token::Char('w', Category::Letter),
                        Token::Char('{', Category::BeginGroup),
                        Token::Char('x', Category::Letter),
                        Token::Char('{', Category::BeginGroup),
                        Token::Char('y', Category::Letter),
                        Token::Char('}', Category::EndGroup),
                        Token::Char('}', Category::EndGroup),
                        Token::Char('z', Category::Letter),
                    ],
                )],
            );
        }

        #[test]
        fn it_succeeds_parsing_when_tokens_match_parameters() {
            try_parsing_replacements(
                &["\\a x\\y%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Token(Token::ControlSequence("y".to_string())),
                ],
            );
        }

        #[test]
        #[should_panic(expected = "Non-matching token found")]
        fn it_fails_parsing_when_tokens_dont_match_parameters() {
            try_parsing_replacements(
                &["\\a xy%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Token(Token::ControlSequence("y".to_string())),
                ],
            );
        }

        #[test]
        fn it_ignores_spaces_when_parsing_undelimited_parameters() {
            assert_parses_to_replacements(
                &["\\a x      y%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Parameter(1),
                ],
                vec![(1, vec![Token::Char('y', Category::Letter)])],
            );
        }

        #[test]
        fn it_parses_delimited_tokens() {
            // Test that it succeeds when the delimiters match immediately
            assert_parses_to_replacements(
                &["\\a x%"],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                ],
                vec![(1, vec![])],
            );

            // Test that it succeeds when the delimiters match after some
            // non-matching tokens
            assert_parses_to_replacements(
                &["\\a zyx%"],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                ],
                vec![(
                    1,
                    vec![
                        Token::Char('z', Category::Letter),
                        Token::Char('y', Category::Letter),
                    ],
                )],
            );

            // Test that it succeeds when the delimiters match after a balanced
            // group
            assert_parses_to_replacements(
                &["\\a {x}yx%"],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                ],
                vec![(
                    1,
                    vec![
                        Token::Char('{', Category::BeginGroup),
                        Token::Char('x', Category::Letter),
                        Token::Char('}', Category::EndGroup),
                        Token::Char('y', Category::Letter),
                    ],
                )],
            );

            // Test that it succeeds when there are multiple delimiters
            assert_parses_to_replacements(
                &["\\a xyzw%"],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('z', Category::Letter)),
                    MacroListElem::Token(Token::Char('w', Category::Letter)),
                ],
                vec![(
                    1,
                    vec![
                        Token::Char('x', Category::Letter),
                        Token::Char('y', Category::Letter),
                    ],
                )],
            );

            // Test that it succeeds when there are other tokens that match the
            // start of the delimiter.
            assert_parses_to_replacements(
                &["\\a xaxabxabc%"],
                vec![
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('a', Category::Letter)),
                    MacroListElem::Token(Token::Char('b', Category::Letter)),
                    MacroListElem::Token(Token::Char('c', Category::Letter)),
                ],
                vec![(
                    1,
                    vec![
                        Token::Char('x', Category::Letter),
                        Token::Char('a', Category::Letter),
                        Token::Char('x', Category::Letter),
                        Token::Char('a', Category::Letter),
                        Token::Char('b', Category::Letter),
                        Token::Char('x', Category::Letter),
                    ],
                )],
            );
        }

        #[test]
        fn it_parses_multiple_parameters() {
            assert_parses_to_replacements(
                &["\\a x a{x}yx {z\\w}%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Parameter(1),
                    MacroListElem::Parameter(2),
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Parameter(3),
                ],
                vec![
                    (1, vec![Token::Char('a', Category::Letter)]),
                    (
                        2,
                        vec![
                            Token::Char('{', Category::BeginGroup),
                            Token::Char('x', Category::Letter),
                            Token::Char('}', Category::EndGroup),
                            Token::Char('y', Category::Letter),
                        ],
                    ),
                    (
                        3,
                        vec![
                            Token::Char('z', Category::Letter),
                            Token::ControlSequence("w".to_string()),
                        ],
                    ),
                ],
            );
        }

        #[test]
        fn it_handles_macros_with_begin_groups() {
            assert_parses_to_replacements(
                &["\\a xabc{%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Parameter(1),
                    MacroListElem::Token(Token::Char('{', Category::BeginGroup)),
                ],
                vec![(
                    1,
                    vec![
                        Token::Char('a', Category::Letter),
                        Token::Char('b', Category::Letter),
                        Token::Char('c', Category::Letter),
                    ],
                )],
            );

            try_parsing_replacements(
                &["\\a x{%"],
                vec![
                    MacroListElem::Token(Token::Char('x', Category::Letter)),
                    MacroListElem::Token(Token::Char('{', Category::BeginGroup)),
                ],
            );
        }
    }
}

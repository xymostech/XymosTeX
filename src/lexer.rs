use crate::category::Category;
use crate::token::Token;

#[derive(Debug, PartialEq, Eq)]
enum LexState {
    BeginningLine,
    MiddleLine,
    SkippingBlanks,
}

#[derive(Debug, PartialEq, Eq)]
enum PlainLexResult {
    Eof,
    Eol,
    Char(char),
}

pub struct Lexer {
    source: Vec<Vec<char>>,

    row: usize,
    col: usize,

    state: LexState,
}

// TODO(emily): pull this out of TeXState instead of doing this hackily here
fn get_category(ch: char) -> Category {
    if ch == '^' {
        Category::Superscript
    } else if ch == '%' {
        Category::Comment
    } else if ch == '\\' {
        Category::Escape
    } else if ch == '\n' {
        Category::EndOfLine
    } else if ch == '{' {
        Category::BeginGroup
    } else if ch == '}' {
        Category::EndGroup
    } else if ch == ' ' {
        Category::Space
    } else if ch == '\u{0000}' {
        Category::Ignored
    } else if ch == '\u{00ff}' {
        Category::Invalid
    } else {
        Category::Letter
    }
}

fn is_hex_char(ch: char) -> bool {
    return ('0' <= ch && ch <= '9') || ('a' <= ch && ch <= 'f');
}

fn hex_value(ch: char) -> u8 {
    if '0' <= ch && ch <= '9' {
        (ch as u8) - ('0' as u8)
    } else if 'a' <= ch && ch <= 'f' {
        (ch as u8) - ('a' as u8) + 10
    } else {
        panic!("Illegal hex char: {}", ch);
    }
}

impl Lexer {
    pub fn new(lines: &[&str]) -> Lexer {
        let source = lines.iter().map(|&s| {
            let mut line = String::from(s);
            line.push('\n');
            return line.chars().collect();
        }).collect();

        return Lexer {
            source: source,
            row: 0,
            col: 0,
            state: LexState::BeginningLine,
        };
    }

    fn get_plain_char(&mut self) -> PlainLexResult {
        if self.row == self.source.len() {
            return PlainLexResult::Eof;
        }

        let line = &self.source[self.row];

        if self.col == line.len() {
            self.row += 1;
            self.col = 0;
            return PlainLexResult::Eol;
        }

        let ch = line[self.col];
        self.col += 1;
        return PlainLexResult::Char(ch);
    }

    fn unget_plain_char(&mut self, ch: &PlainLexResult) {
        match ch {
            PlainLexResult::Char(_) => self.col -= 1,
            PlainLexResult::Eol => {
                self.row -= 1;
                self.col = self.source[self.row].len() - 1;
            },
            PlainLexResult::Eof => (),
        }
    }

    fn get_char(&mut self) -> PlainLexResult {
        match self.get_plain_char() {
            PlainLexResult::Char(ch) => self.handle_trigraphs(ch),
            rest => rest,
        }
    }

    fn handle_trigraphs(&mut self, first_char: char) -> PlainLexResult {
        let first_result = PlainLexResult::Char(first_char);

        if get_category(first_char) != Category::Superscript{
            return first_result;
        }

        let second_char: char = match self.get_plain_char() {
            PlainLexResult::Char(ch) => ch,
            rest => {
                self.unget_plain_char(&rest);
                return first_result;
            },
        };

        if get_category(second_char) != Category::Superscript {
            self.unget_plain_char(&PlainLexResult::Char(second_char));
            return first_result;
        }

        let third_char: char = match self.get_plain_char() {
            PlainLexResult::Char(ch) => ch,
            rest => {
                self.unget_plain_char(&rest);
                self.unget_plain_char(&PlainLexResult::Char(second_char));
                return first_result;
            },
        };

        match self.get_plain_char() {
            PlainLexResult::Char(fourth_char) => {
                let final_char =
                    if is_hex_char(third_char) && is_hex_char(fourth_char) {
                        (hex_value(third_char) * 16 + hex_value(fourth_char)) as char
                    } else {
                        self.unget_plain_char(&PlainLexResult::Char(fourth_char));
                        if third_char <= '?' {
                            ((third_char as u8) + 0x40) as char
                        } else {
                            ((third_char as u8) - 0x40) as char
                        }
                    };
                self.handle_trigraphs(final_char)
            },
            rest => {
                self.unget_plain_char(&rest);
                let final_char = if third_char <= '?' {
                    ((third_char as u8) + 0x40) as char
                } else {
                    ((third_char as u8) - 0x40) as char
                };
                self.handle_trigraphs(final_char)
            }
        }
    }

    pub fn lex_token(&mut self) -> Option<Token> {
        match self.get_char() {
            PlainLexResult::Eof => None,
            PlainLexResult::Eol => {
                self.state = LexState::BeginningLine;
                self.lex_token()
            },
            PlainLexResult::Char(c) => {
                match get_category(c) {
                    Category::Invalid => panic!("Invalid character: '{}'", c),
                    Category::Escape => {
                        self.state = LexState::SkippingBlanks;

                        let first_char = match self.get_char() {
                            PlainLexResult::Char(c) => c,
                            _ => panic!("Invalid EOF or EOL lexing control sequence"),
                        };

                        match get_category(first_char) {
                            Category::Letter => {
                                let mut sequence = first_char.to_string();

                                loop {
                                    match self.get_char() {
                                        PlainLexResult::Char(c)
                                            if get_category(c) == Category::Letter =>
                                            sequence.push(c),

                                        rest => {
                                            self.unget_plain_char(&rest);
                                            break;
                                        }
                                    }
                                }

                                Some(Token::ControlSequence(sequence))
                            },
                            _ => {
                                Some(Token::ControlSequence(first_char.to_string()))
                            },
                        }
                    },
                    Category::EndOfLine => {
                        match self.state {
                            LexState::BeginningLine => {
                                Some(Token::ControlSequence("par".to_string()))
                            },
                            LexState::MiddleLine => {
                                Some(Token::Char(' ', Category::Space))
                            },
                            LexState::SkippingBlanks => {
                                self.lex_token()
                            },
                        }
                    },
                    Category::Space => {
                        if self.state == LexState::MiddleLine {
                            self.state = LexState::SkippingBlanks;
                            Some(Token::Char(' ', Category::Space))
                        } else {
                            self.lex_token()
                        }
                    },
                    Category::Comment => {
                        self.col = self.source[self.row].len();
                        self.lex_token()
                    },
                    Category::Ignored => {
                        self.lex_token()
                    },
                    cat => {
                        self.state = LexState::MiddleLine;
                        Some(Token::Char(c, cat))
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_lexes_to(lines: &[&str], expected_toks: &[Token]) {
        let mut lexer = Lexer::new(lines);

        let mut real_toks = Vec::new();

        while let Some(tok) = lexer.lex_token() {
            real_toks.push(tok);
        }

        assert_eq!(expected_toks, &real_toks[..]);
    }

    #[test]
    fn it_lexes_char_tokens() {
        assert_lexes_to(
            &["a%"],
            &[Token::Char('a', Category::Letter)]);
    }

    #[test]
    fn it_lexes_multiple_tokens() {
        assert_lexes_to(
            &["ab%"],
            &[Token::Char('a', Category::Letter),
              Token::Char('b', Category::Letter)]);
    }

    #[test]
    fn it_lexes_control_sequences() {
        assert_lexes_to(
            &["\\ab%"],
            &[Token::ControlSequence("ab".to_string())]);
        assert_lexes_to(
            &["\\@%"],
            &[Token::ControlSequence("@".to_string())]);
    }

    #[test]
    fn it_ignores_ignored_tokens() {
        assert_lexes_to(
            &["a\u{0000}b%"],
            &[Token::Char('a', Category::Letter),
              Token::Char('b', Category::Letter)]);
    }

    #[test]
    #[should_panic(expected="Invalid character:")]
    fn it_panics_on_invalid_tokens() {
        assert_lexes_to(
            &["\u{00ff}"],
            &[]);
    }

    #[test]
    fn it_lexes_char_trigraphs() {
        assert_lexes_to(
            &["^^:%"],
            &[Token::Char('z', Category::Letter)]);
    }

    #[test]
    fn it_lexes_trigraphs_recursively() {
        assert_lexes_to(
            &["^^\u{001e}^:%"],
            &[Token::Char('z', Category::Letter)]);
    }

    #[test]
    fn it_lexes_hex_trigraphs() {
        // ^^7a is a valid hex trigraph which decodes to z
        assert_lexes_to(
            &["^^7a%"],
            &[Token::Char('z', Category::Letter)]);

        // g isn't a hex char, so ^^7g should be interpreted as a ^^7 trigraph
        // and the character g.
        assert_lexes_to(
            &["^^7g%"],
            &[Token::Char('w', Category::Letter),
              Token::Char('g', Category::Letter)]);
    }

    #[test]
    fn it_ignores_leading_spaces() {
        assert_lexes_to(
            &["  a%"],
            &[Token::Char('a', Category::Letter)]);
    }

    #[test]
    fn it_includes_trailing_spaces() {
        assert_lexes_to(
            &["a "],
            &[Token::Char('a', Category::Letter),
              Token::Char(' ', Category::Space)]);
    }

    #[test]
    fn it_ignores_space_after_control_sequence() {
        assert_lexes_to(
            &["\\a \\abc \\  %"],
            &[Token::ControlSequence("a".to_string()),
              Token::ControlSequence("abc".to_string()),
              Token::ControlSequence(" ".to_string())]);
    }

    #[test]
    fn it_condenses_multiple_spaces_into_one_space() {
        assert_lexes_to(
            &[" a  ", " a%"],
            &[Token::Char('a', Category::Letter),
              Token::Char(' ', Category::Space),
              Token::Char('a', Category::Letter)]);
    }

    #[test]
    fn it_converts_double_newlines_to_pars() {
        assert_lexes_to(
            &["a%", "", "a%"],
            &[Token::Char('a', Category::Letter),
              Token::ControlSequence("par".to_string()),
              Token::Char('a', Category::Letter)]);
    }

    #[test]
    fn it_ignores_comments() {
        assert_lexes_to(
            &["a%b"],
            &[Token::Char('a', Category::Letter)]);
    }
}

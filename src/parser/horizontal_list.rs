use crate::category::Category;
use crate::token::Token;
use crate::parser::Parser;
use crate::state::TeXState;

#[derive(Debug, PartialEq, Eq)]
enum HorizontalListElem {
    Char(char),
}

impl<'a> Parser<'a> {
    fn parse_horizontal_list_elem(&mut self, group_level: &mut usize) -> Option<HorizontalListElem> {
        match self.lexer.lex_token() {
            None => None,
            Some(Token::Char(ch, cat)) => {
                match cat {
                    Category::Letter => Some(HorizontalListElem::Char(ch)),
                    Category::Other => Some(HorizontalListElem::Char(ch)),
                    Category::Space => Some(HorizontalListElem::Char(' ')),
                    Category::BeginGroup => {
                        *group_level += 1;
                        // TODO(emily): save state
                        self.parse_horizontal_list_elem(group_level)
                    },
                    Category::EndGroup => {
                        if *group_level == 0 {
                            None
                        } else {
                            *group_level -= 1;
                            // TODO(emily): pop state
                            self.parse_horizontal_list_elem(group_level)
                        }
                    },
                    _ => panic!("unimplemented"),
                }
            },
            Some(Token::ControlSequence(seq)) => {
                if seq == "par" {
                    Some(HorizontalListElem::Char(' '))
                } else {
                    panic!("unimplemented");
                }
            },
        }
    }

    fn parse_horizontal_list_to_elems(&mut self) -> Vec<HorizontalListElem> {
        let mut result = Vec::new();

        let mut group_level = 0;
        while let Some(elem) = self.parse_horizontal_list_elem(&mut group_level) {
            result.push(elem);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parses_to(lines: &[&str], expected_toks: &[HorizontalListElem]) {
        let state = TeXState::new();
        let mut parser = Parser::new(lines, &state);

        assert_eq!(
            parser.parse_horizontal_list_to_elems(),
            expected_toks);
    }

    #[test]
    fn it_parses_letters() {
        assert_parses_to(
            &["ab%"],
            &[HorizontalListElem::Char('a'),
              HorizontalListElem::Char('b')]);
    }

    #[test]
    fn it_parses_grouping() {
        assert_parses_to(
            &["a{b}c%"],
            &[HorizontalListElem::Char('a'),
              HorizontalListElem::Char('b'),
              HorizontalListElem::Char('c')]);
    }
}

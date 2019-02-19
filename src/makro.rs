// NOTE(xymostech): this file is called makro because macro is a reserved
// keyword.

use std::collections::HashMap;

use crate::token::Token;

#[derive(PartialEq, Eq, Debug)]
pub enum MacroListElem {
    Token(Token),
    Parameter(usize),
}

#[derive(PartialEq, Eq, Debug)]
pub struct Macro {
    parameter_list: Vec<MacroListElem>,
    replacement_list: Vec<MacroListElem>,
}

impl Macro {
    pub fn new(parameter_list: Vec<MacroListElem>, replacement_list: Vec<MacroListElem>) -> Macro {
        let makro: Macro = Macro {
            parameter_list: parameter_list,
            replacement_list: replacement_list,
        };

        makro.validate();

        makro
    }

    fn validate(&self) {
        // The parameters in the parameter list need to be in order, so we make
        // sure that's the case.
        let mut num_parameters: usize = 0;
        for elem in self.parameter_list.iter() {
            if let MacroListElem::Parameter(param_num) = elem {
                num_parameters += 1;
                if *param_num != num_parameters {
                    panic!(
                        "Out-of-order parameter in macro parameter list: {}",
                        param_num
                    );
                }
            }
        }

        // Also, all of the parameters in the replacement text must be in the
        // parameters, so we make sure that's true as well.
        for elem in self.replacement_list.iter() {
            if let MacroListElem::Parameter(param_num) = elem {
                if *param_num > num_parameters {
                    panic!(
                        "Parameter in replacement text outside of range: {}",
                        param_num
                    );
                }
            }
        }
    }

    pub fn get_replacement(&self, parameter_values: &HashMap<usize, Vec<Token>>) -> Vec<Token> {
        self.replacement_list
            .iter()
            .flat_map(|elem| match elem {
                MacroListElem::Parameter(param_num) => match parameter_values.get(param_num) {
                    Some(tok_list) => tok_list.clone(),
                    None => panic!("Missing parameter in replacement: {}", param_num),
                },
                MacroListElem::Token(tok) => vec![tok.clone()],
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Out-of-order parameter")]
    fn it_errors_with_incorrect_parameter_list() {
        Macro::new(
            vec![MacroListElem::Parameter(2), MacroListElem::Parameter(1)],
            vec![],
        );
    }

    #[test]
    #[should_panic(expected = "Parameter in replacement text outside of")]
    fn it_errors_with_incorrect_replacement_list() {
        Macro::new(
            vec![MacroListElem::Parameter(1)],
            vec![MacroListElem::Parameter(1), MacroListElem::Parameter(2)],
        );
    }

    #[test]
    fn it_correctly_generates_replacements() {
        let makro = Macro::new(
            vec![MacroListElem::Parameter(1), MacroListElem::Parameter(2)],
            vec![
                MacroListElem::Parameter(2),
                MacroListElem::Token(Token::ControlSequence("boo".to_string())),
                MacroListElem::Parameter(1),
            ],
        );

        let mut replacements = HashMap::new();
        replacements.insert(1, vec![Token::ControlSequence("c".to_string())]);
        replacements.insert(
            2,
            vec![
                Token::ControlSequence("a".to_string()),
                Token::ControlSequence("b".to_string()),
            ],
        );

        assert_eq!(
            vec![
                Token::ControlSequence("a".to_string()),
                Token::ControlSequence("b".to_string()),
                Token::ControlSequence("boo".to_string()),
                Token::ControlSequence("c".to_string())
            ],
            makro.get_replacement(&replacements)
        );
    }
}

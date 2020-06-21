#[derive(Clone, Debug, PartialEq)]
enum MathClass {
    Ordinary = 0,
    LargeOperator = 1,
    BinaryOperation = 2,
    Relation = 3,
    Opening = 4,
    Closing = 5,
    Punctuation = 6,
    VariableFamily = 7,
    Active = 8,
}

impl MathClass {
    fn from_number(num: u8) -> MathClass {
        match num {
            0 => MathClass::Ordinary,
            1 => MathClass::LargeOperator,
            2 => MathClass::BinaryOperation,
            3 => MathClass::Relation,
            4 => MathClass::Opening,
            5 => MathClass::Closing,
            6 => MathClass::Punctuation,
            7 => MathClass::VariableFamily,
            _ => panic!("Invalid class {}", num),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MathCode {
    class: MathClass,
    family: u8,
    position: u8,
}

impl MathCode {
    pub fn from_number(num: u32) -> MathCode {
        if num > 0x8000 {
            panic!("Invalid value for math code: {}, should be in the range 0..32768", num);
        }

        if num == 0x8000 {
            return MathCode {
                class: MathClass::Active,
                family: 0,
                position: 0,
            };
        }

        let class = (num / 0x1000) as u8;
        let family = ((num / 0x100) % 0x10) as u8;
        let position = (num % 0x100) as u8;

        MathCode {
            class: MathClass::from_number(class),
            family,
            position,
        }
    }
}

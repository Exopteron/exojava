use crate::lexer::{LexingError, LexErrorType};

use super::{Tokenizable, stream::JavaTerminalStream, early::FinalTerminalElement};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operator {
    EqAssign,
    Gt,
    Lt,
    Not,
    GraveOp,
    Question,
    Colon,
    LambdaCreate,
    EqCompare,
    GtEq,
    LtEq,
    NotEq,
    AndLogical,
    OrLogical,
    PlusPlus,
    MinusMinus,
    Plus,
    Minus,
    Multiply,
    Divide,
    AndBitwise,
    OrBitwise,
    Xor,
    Modulo,
    LShift,
    RShift,
    RshiftUnSigned,
    PlusEquals,
    MinusEquals,
    MultiplyEquals,
    DivideEquals,
    AndEquals,
    OrEquals,
    XorEquals,
    ModuloEquals,
    LShiftEquals,
    RShiftEquals,
    RShiftUnsignedEquals
}

impl Operator {
    pub fn lookahead_operator(s: &mut JavaTerminalStream) -> bool {
        match s.lookahead() {
            Ok(v) => match v {
                FinalTerminalElement::LineTerminator(_) => false,
                FinalTerminalElement::InputCharacter(v) => {
                    matches!(v.0, '=' | '>' | '<' | '!' | '~' | '?' | ':' | '-' | '&' | '|' | '+' | '*' | '/' | '^' | '%')
                }
            }
            Err(_) => false
        }
    }
}

impl Tokenizable for Operator {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if Self::lookahead_operator(s) {
            Ok(match s.match_input_character()?.0 {
                '!' => {
                    if s.match_str("=", true).is_ok() {
                        Self::NotEq
                    } else {
                        Self::Not
                    }
                },
                '&' => {
                    if s.match_str("&", true).is_ok() {
                        Self::AndLogical
                    } else if s.match_str("=", true).is_ok() {
                        Self::AndEquals
                    } else {
                        Self::AndBitwise
                    }
                },
                '=' => {
                    if s.match_str("=", true).is_ok() {
                        Self::EqCompare
                    } else {
                        Self::EqAssign
                    }
                },
                '>' => {
                    if s.match_str("=", true).is_ok() {
                        Self::GtEq
                    } else if s.match_str(">", true).is_ok() {
                        if s.match_str(">", true).is_ok() {
                            if s.match_str("=", true).is_ok() {
                                Self::RShiftUnsignedEquals
                            } else {
                                Self::RshiftUnSigned
                            }
                        } else if s.match_str("=", true).is_ok() {
                            Self::RShiftEquals
                        } else {
                            Self::RShift
                        }
                    } else {
                        Self::Gt
                    }
                }
                '<' => {
                    if s.match_str("=", true).is_ok() {
                        Self::LtEq
                    } else if s.match_str("<", true).is_ok() {
                        if s.match_str("=", true).is_ok() {
                            Self::LShiftEquals
                        } else {
                            Self::LShift
                        }
                    } else {
                        Self::Lt
                    }
                }
                '?' => Self::Question,
                ':' => Self::Colon,
                '~' => Self::GraveOp,
                '-' => {
                    if s.match_str(">", true).is_ok() {
                        Self::LambdaCreate
                    } else if s.match_str("-", true).is_ok() {
                        Self::MinusMinus
                    } else if s.match_str("=", true).is_ok() {
                        Self::MinusEquals
                    } else {
                        Self::Minus
                    }
                }
                '+' => {
                    if s.match_str("+", true).is_ok() {
                        Self::PlusPlus
                    } else if s.match_str("=", true).is_ok() {
                        Self::PlusEquals
                    } else {
                        Self::Plus
                    }
                }
                '*' => {
                    if s.match_str("=", true).is_ok() {
                        Self::MultiplyEquals
                    } else {
                        Self::Multiply
                    }
                }
                '/' => {
                    if s.match_str("=", true).is_ok() {
                        Self::DivideEquals
                    } else {
                        Self::Divide
                    }
                }
                '|' => {
                    if s.match_str("|", true).is_ok() {
                        Self::OrLogical
                    } else if s.match_str("=", true).is_ok() {
                        Self::OrEquals
                    } else {
                        Self::OrBitwise
                    }
                },
                '^' => {
                    if s.match_str("=", true).is_ok() {
                        Self::XorEquals
                    } else {
                        Self::Xor
                    }
                }
                '%' => {
                    if s.match_str("=", true).is_ok() {
                        Self::ModuloEquals
                    } else {
                        Self::Modulo
                    }
                }
                v => return Err(LexingError::new(LexErrorType::SyntaxError(format!("Unknown separator {}", v)), (s.position, s.position)))
            })
        } else {
            Err(LexingError::new(LexErrorType::SyntaxError("Unknown separator".to_string()), (s.position, s.position)))
        }
    }
}
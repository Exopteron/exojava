use std::rc::Rc;

pub mod number;
use crate::lexer::{LexErrorType, LexingError};

use self::number::integer::IntegerLiteral;

use super::{early::{FinalTerminalElement}, Tokenizable, Token};

#[derive(Clone, Debug)]
pub enum Literal {
    NullLiteral,
    BooleanLiteral(bool),
    StringLiteral(Token<StringLiteral>),
    CharacterLiteral(Token<CharacterLiteral>),
    IntegerLiteral(Token<IntegerLiteral>)
}

impl Tokenizable for Literal {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("null", true).is_ok() {
            Ok(Self::NullLiteral)
        } else if s.match_str("true", true).is_ok() {
            Ok(Self::BooleanLiteral(true))
        } else if s.match_str("false", true).is_ok() {
            Ok(Self::BooleanLiteral(false))
        } else if s.match_str("\"", false).is_ok() {
            Ok(Self::StringLiteral(s.get()?))
        } else if s.match_str("'", false).is_ok() {
            Ok(Self::CharacterLiteral(s.get()?))
        } else if let Ok(v) = s.get::<IntegerLiteral>() {
            Ok(Self::IntegerLiteral(v))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("Bad literal".to_string()),
                (s.position, s.position),
            ))
        }
    }
}


#[derive(Clone, Debug)]
pub struct StringLiteral(pub Rc<String>);

impl Tokenizable for StringLiteral {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_str("\"", true)?;
        let mut str = String::new();
        while s.match_str("\"", true).is_err() {
            if let Ok(v) = s.get::<EscapeSequence>() {
                str.push(v.v.0);
            } else {
                str.push(s.match_input_character()?.0);
            }
        }
        Ok(Self(Rc::new(str)))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterLiteral(pub char);

impl Tokenizable for CharacterLiteral {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_str("\'", true)?;
        let c;
        if let Ok(v) = s.get::<EscapeSequence>() {
            c = v.v.0;
        } else {
            c = s.match_input_character()?.0;
        }
        s.match_str("\'", true)?;
        Ok(Self(c))
    }
}



#[derive(Clone, Copy, Debug)]
pub struct EscapeSequence(pub char);

impl Tokenizable for EscapeSequence {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_str("\\", true)?;
        if let FinalTerminalElement::InputCharacter(v) = s.lookahead()? {
            let mut flag = true;
            let v = match v.0 {
                'b' => Ok(Self('\u{0008}')),
                't' => Ok(Self('\u{0009}')),
                'n' => Ok(Self('\u{000a}')),
                'f' => Ok(Self('\u{000c}')),
                'r' => Ok(Self('\u{000d}')),
                '"' => Ok(Self('\u{0022}')),
                '\'' => Ok(Self('\u{0027}')),
                '\\' => Ok(Self('\u{005c}')),
                _ => {
                    flag = false;
                    let mut digit = String::new();
                    if let Ok(a) = s.get::<ZeroToThree>() {
                        digit.push(a.v.0);
                        if let Ok(b) = s.get::<OctalDigit>() {
                            digit.push(b.v.0);
                            if let Ok(c) = s.get::<OctalDigit>() {
                                digit.push(c.v.0);
                            }
                        }
                    } else if let Ok(a) = s.get::<OctalDigit>() {
                        digit.push(a.v.0);
                        if let Ok(b) = s.get::<OctalDigit>() {
                            digit.push(b.v.0);
                        }
                    }
                    if digit.is_empty() {
                        Err(LexingError::new(
                            LexErrorType::SyntaxError("Bad octal escape".to_string()),
                            (s.position, s.position),
                        ))
                    } else {
                        Ok(Self(u8::from_str_radix(&digit, 8).map_err(|v| {
                            LexingError::new(
                                LexErrorType::ParseIntError(v),
                                (s.position, s.position),
                            )
                        })? as char))
                    }
                }
            };
            if flag {
                s.next()?;
            }
            v
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("Line terminator in escape sequence".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OctalDigit(pub char);

impl Tokenizable for OctalDigit {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("0", true).is_ok() {
            Ok(Self('0'))
        } else if s.match_str("1", true).is_ok() {
            Ok(Self('1'))
        } else if s.match_str("2", true).is_ok() {
            Ok(Self('2'))
        } else if s.match_str("3", true).is_ok() {
            Ok(Self('3'))
        } else if s.match_str("4", true).is_ok() {
            Ok(Self('4'))
        } else if s.match_str("5", true).is_ok() {
            Ok(Self('5'))
        } else if s.match_str("6", true).is_ok() {
            Ok(Self('6'))
        } else if s.match_str("7", true).is_ok() {
            Ok(Self('7'))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid octaldigit".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZeroToThree(pub char);

impl Tokenizable for ZeroToThree {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if s.match_str("0", true).is_ok() {
            Ok(Self('0'))
        } else if s.match_str("1", true).is_ok() {
            Ok(Self('1'))
        } else if s.match_str("2", true).is_ok() {
            Ok(Self('2'))
        } else if s.match_str("3", true).is_ok() {
            Ok(Self('3'))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid zerotothree".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

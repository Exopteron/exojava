use std::rc::Rc;

use crate::lexer::{
    tokens::{stream::JavaTerminalStream, Token, Tokenizable},
    LexErrorType, LexingError,
};

#[derive(Debug, Clone)]
pub enum IntegerLiteral {
    DecimalIntegerLiteral(Token<DecimalIntegerLiteral>),
}

impl Tokenizable for IntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let d = s.position;
        if let Ok(v) = s.get::<DecimalIntegerLiteral>() {
            return Ok(Self::DecimalIntegerLiteral(v));
        } else {
            s.position = d;
        }
        Err(LexingError::new(
            LexErrorType::SyntaxError("Bad integer literal".to_string()),
            (s.position, s.position),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DecimalIntegerLiteral {
    pub data: Token<DecimalNumeral>,
    pub type_suffix: Option<Token<IntegerTypeSuffix>>,
}
impl Tokenizable for DecimalIntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let data = s.get::<DecimalNumeral>()?;
        let type_suffix = s.get::<IntegerTypeSuffix>().ok();
        Ok(Self { data, type_suffix })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IntegerTypeSuffix {
    Long,
}

impl Tokenizable for IntegerTypeSuffix {
    fn parse(
        s: &mut crate::lexer::tokens::stream::JavaTerminalStream,
    ) -> crate::lexer::LexResult<Self> {
        if s.match_str("l", true).is_ok() || s.match_str("L", true).is_ok() {
            Ok(Self::Long)
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("Invalid integer type suffix".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecimalNumeral(pub Rc<String>);
impl Tokenizable for DecimalNumeral {
    fn parse(
        s: &mut crate::lexer::tokens::stream::JavaTerminalStream,
    ) -> crate::lexer::LexResult<Self> {
        if s.match_str("0", true).is_ok() {
            Ok(Self(Rc::new("0".to_string())))
        } else {
            let mut str = String::new();
            loop {
                if s.match_str("_", true).is_ok() {
                    continue;
                }
                if let Ok(v) = s.get::<DecimalDigit>() {
                    str.push(v.v.0);
                    continue;
                }
                break;
            }
            if str.is_empty() {
                return Err(LexingError::new(
                    LexErrorType::SyntaxError("Bad decimal numeral".to_string()),
                    (s.position, s.position),
                ));
            }
            Ok(Self(Rc::new(str)))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DecimalDigit(pub char);

impl Tokenizable for DecimalDigit {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
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
        } else if s.match_str("8", true).is_ok() {
            Ok(Self('8'))
        } else if s.match_str("9", true).is_ok() {
            Ok(Self('9'))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid decimaldigit".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

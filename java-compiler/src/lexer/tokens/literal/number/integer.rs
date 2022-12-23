use std::rc::Rc;

use crate::lexer::{
    tokens::{stream::JavaTerminalStream, Token, Tokenizable, literal::OctalDigit},
    LexErrorType, LexingError, LexResult,
};



#[derive(Debug, Clone)]
pub enum IntegerLiteral {
    Long(i64),
    Int(u32), // u32 because -2147483648
}


fn to_integer(s: &JavaTerminalStream, suffix: Option<IntegerTypeSuffix>, str: &str, radix: u32) -> LexResult<IntegerLiteral> {
    match suffix {
        Some(IntegerTypeSuffix::Long) => {
            Ok(IntegerLiteral::Long(i64::from_str_radix(str, radix).map_err(
                |v| {
                    LexingError::new(
                        LexErrorType::ParseIntError(v),
                        (s.position, s.position),
                    )
                },
            )?))
        }
        None => {
            Ok(IntegerLiteral::Int(u32::from_str_radix(str, radix).map_err(
                |v| {
                    LexingError::new(
                        LexErrorType::ParseIntError(v),
                        (s.position, s.position),
                    )
                },
            )?))
        }
    }
}

impl Tokenizable for IntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {

        if HexNumeral::lookahead(s) {
            let v = s.get::<HexIntegerLiteral>()?;
            let v = v.v;
            to_integer(s, v.type_suffix.map(|v| v.v), &v.data.0, 16)
        } else if BinaryNumeral::lookahead(s) {
            let v = s.get::<BinaryIntegerLiteral>()?;
            let v = v.v;
            to_integer(s, v.type_suffix.map(|v| v.v), &v.data.0, 2)
        } else {
            let d = s.position;
            if let Ok(v) = s.get::<OctalIntegerLiteral>() {
                let v = v.v;
                return to_integer(s, v.type_suffix.map(|v| v.v), &v.data.0, 8)
            } else {
                s.position = d;
            }


            if let Ok(v) = s.get::<DecimalIntegerLiteral>() {
                let v = v.v;
                return to_integer(s, v.type_suffix.map(|v| v.v), &v.data.0, 10)
            } else {
                s.position = d;
            }


            Err(LexingError::new(
                LexErrorType::SyntaxError("Bad integer literal".to_string()),
                (s.position, s.position),
            ))
        }

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
pub struct HexIntegerLiteral {
    pub data: Token<HexNumeral>,
    pub type_suffix: Option<Token<IntegerTypeSuffix>>,
}
impl Tokenizable for HexIntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let data = s.get::<HexNumeral>()?;
        let type_suffix = s.get::<IntegerTypeSuffix>().ok();
        Ok(Self { data, type_suffix })
    }
}

#[derive(Debug, Clone)]
pub struct HexNumeral(pub Rc<String>);
impl HexNumeral {
    pub fn lookahead(s: &mut JavaTerminalStream) -> bool {
        s.match_one_of(&["0x", "0X"], false).is_ok()
    }
}

impl Tokenizable for HexNumeral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_one_of(&["0x", "0X"], true)?;
        let mut str = String::new();
        loop {
            if s.match_str("_", true).is_ok() {
                continue;
            }
            if let Ok(v) = s.get::<HexDigit>() {
                str.push(v.v.0);
                continue;
            }
            break;
        }
        if str.is_empty() {
            return Err(LexingError::new(
                LexErrorType::SyntaxError("Bad hex numeral".to_string()),
                (s.position, s.position),
            ));
        }
        Ok(Self(Rc::new(str)))
    }
}


#[derive(Debug, Clone)]
pub struct OctalIntegerLiteral {
    pub data: Token<OctalNumeral>,
    pub type_suffix: Option<Token<IntegerTypeSuffix>>,
}
impl Tokenizable for OctalIntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let data = s.get::<OctalNumeral>()?;
        let type_suffix = s.get::<IntegerTypeSuffix>().ok();
        Ok(Self { data, type_suffix })
    }
}

#[derive(Debug, Clone)]
pub struct OctalNumeral(pub Rc<String>);

impl Tokenizable for OctalNumeral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_one_of(&["0"], true)?;
        let mut str = String::new();
        loop {
            if s.match_str("_", true).is_ok() {
                continue;
            }
            if let Ok(v) = s.get::<OctalDigit>() {
                str.push(v.v.0);
                continue;
            }
            break;
        }
        if str.is_empty() {
            return Err(LexingError::new(
                LexErrorType::SyntaxError("Bad octal numeral".to_string()),
                (s.position, s.position),
            ));
        }
        Ok(Self(Rc::new(str)))
    }
}



#[derive(Debug, Clone)]
pub struct BinaryIntegerLiteral {
    pub data: Token<BinaryNumeral>,
    pub type_suffix: Option<Token<IntegerTypeSuffix>>,
}
impl Tokenizable for BinaryIntegerLiteral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let data = s.get::<BinaryNumeral>()?;
        let type_suffix = s.get::<IntegerTypeSuffix>().ok();
        Ok(Self { data, type_suffix })
    }
}

#[derive(Debug, Clone)]
pub struct BinaryNumeral(pub Rc<String>);
impl BinaryNumeral {
    pub fn lookahead(s: &mut JavaTerminalStream) -> bool {
        s.match_one_of(&["0b", "0B"], false).is_ok()
    }
}

impl Tokenizable for BinaryNumeral {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        s.match_one_of(&["0b", "0B"], true)?;
        let mut str = String::new();
        loop {
            if s.match_str("_", true).is_ok() {
                continue;
            }
            if let Ok(v) = s.get::<BinaryDigit>() {
                str.push(v.v.0);
                continue;
            }
            break;
        }
        if str.is_empty() {
            return Err(LexingError::new(
                LexErrorType::SyntaxError("Bad binary numeral".to_string()),
                (s.position, s.position),
            ));
        }
        Ok(Self(Rc::new(str)))
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
        let a = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
        if let Ok(idx) = s.match_one_of(&a, true) {
            Ok(Self(a[idx].chars().next().unwrap()))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid decimaldigit".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HexDigit(pub char);

impl Tokenizable for HexDigit {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let a = [
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "b", "c", "d", "e", "f", "A",
            "B", "C", "D", "E", "F",
        ];
        if let Ok(idx) = s.match_one_of(&a, true) {
            Ok(Self(a[idx].chars().next().unwrap()))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid hexdigit".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinaryDigit(pub char);

impl Tokenizable for BinaryDigit {
    fn parse(s: &mut JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        let a = ["0", "1"];
        if let Ok(idx) = s.match_one_of(&a, true) {
            Ok(Self(a[idx].chars().next().unwrap()))
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("invalid binarydigit".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

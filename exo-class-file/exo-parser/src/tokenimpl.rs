use crate::{error::{ParsingError, ParsingErrorType}, tokens::{Lexer, Parseable}, parse_err, LexerRef, LexerStream};

use super::error::Result;
#[derive(Debug)]
pub struct Alphanumeric(pub char);
impl Parseable for Alphanumeric {
    fn parse(s: &mut LexerStream) -> Result<Self> {
        let c = s.char()?;
        if !c.is_alphanumeric() {
            return Err(parse_err!(s, "not alphanumeric"));
        }
        Ok(Self(c))
    }

}



#[derive(Debug)]
pub struct Numeric(pub char);
impl Parseable for Numeric {
    fn parse(s: &mut LexerStream) -> Result<Self> {
        let c = s.char()?;
        if !c.is_numeric() {
            return Err(parse_err!(s, "not numeric"));
        }
        Ok(Self(c))
    }


}

#[derive(Debug)]
pub struct Whitespace;
impl Parseable for Whitespace {
    fn parse(s: &mut LexerStream) -> Result<Self> {
        loop {
            let p = s.position;
            if let Ok(c) = s.char() {
                if !c.is_whitespace() {
                    s.position = p;
                    return Ok(Self);
                }
            } else {
                return Ok(Self);
            }
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Char<const C: char>;
impl<const C: char> Parseable for Char<C> {
    fn parse(s: &mut LexerStream) -> Result<Self> {
        let c = s.char()?;
        if c != C {
            return Err(parse_err!(s, format!("incorrect character, expected {} but got {}", C, match c {
                '\n' => "newline".to_string(),
                c => c.to_string()
            })));
        }
        Ok(Self)
    }

}


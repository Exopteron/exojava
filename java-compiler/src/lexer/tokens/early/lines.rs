use crate::lexer::{LexingError, LexErrorType, LexResult};
use crate::lexer::tokens::early::CharStream;

use super::BaseToken;

#[derive(Clone, Copy, Debug)]
pub enum LineTerminator {
    NewLine,
    CarriageReturn,
    CarriageReturnLineFeed,
}

impl BaseToken for LineTerminator {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        if s.match_str("\n").is_ok() {
            Ok(Self::NewLine)
        } else if s.match_str("\r\n").is_ok() {
            Ok(Self::CarriageReturnLineFeed)
        } else if s.match_str("\r").is_ok() {
            Ok(Self::CarriageReturn)
        } else {
            Err(LexingError::new(
                LexErrorType::SyntaxError("could not parse line terminator".to_string()),
                (s.position, s.position),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InputCharacter(pub char);

impl BaseToken for InputCharacter {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        if s.match_str("\r").is_ok() || s.match_str("\n").is_ok() {
            Err(LexingError::new(LexErrorType::SyntaxError("CR or LF in input character".to_string()), (s.position, s.position)))
        } else {
            Ok(Self(s.next()?))
        }
    }
}


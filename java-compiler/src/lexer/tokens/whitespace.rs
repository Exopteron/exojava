use crate::lexer::{LexingError, LexErrorType};

use super::{early::{lines::LineTerminator, FinalTerminalElement}, Tokenizable};

#[derive(Debug, Clone, Copy)]
pub enum Whitespace {
    Space,
    HorizontalTab,
    FormFeed,
    LineTerminator(LineTerminator)
}

impl Tokenizable for Whitespace {
    fn parse(s: &mut super::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        match s.next()? {
            FinalTerminalElement::LineTerminator(v) => Ok(Self::LineTerminator(v)),
            FinalTerminalElement::InputCharacter(v) => {
                const FORM_FEED: char = 0xcu8 as char;
                match v.0 {
                    ' ' => Ok(Self::Space),
                    '\t' => Ok(Self::HorizontalTab),
                    FORM_FEED => Ok(Self::FormFeed),
                    v => Err(LexingError::new(LexErrorType::SyntaxError(format!("Non-whitespace character {}", v)), (s.position, s.position)))
                }
            }
        }
    }
}
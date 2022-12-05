use crate::lexer::{LexingError, LexErrorType};

use super::{Tokenizable, stream::JavaTerminalStream, early::FinalTerminalElement};

#[derive(Clone, Copy, Debug)]
pub enum Separator {
    LParenthesis,
    RParenthesis,
    LCurlyBracket,
    RCurlyBracket,
    LSquareBracket,
    RSquareBracket,
    Semicolon,
    Comma,
    Period,
    ThreePeriods,
    At,
    TwoColons
}

impl Separator {
    pub fn lookahead_separator(s: &mut JavaTerminalStream) -> bool {
        match s.lookahead() {
            Ok(v) => match v {
                FinalTerminalElement::LineTerminator(_) => false,
                FinalTerminalElement::InputCharacter(v) => {
                    matches!(v.0, '(' | ')' | '{' | '}' | '[' | ']' | ';' | ',' | '.' | '@' | ':')
                }
            }
            Err(_) => false
        }
    }
}

impl Tokenizable for Separator {
    fn parse(s: &mut super::stream::JavaTerminalStream) -> crate::lexer::LexResult<Self> {
        if Self::lookahead_separator(s) {
            Ok(match s.match_input_character()?.0 {
                '(' => Self::LParenthesis,
                ')' => Self::RParenthesis,
                '{' => Self::LCurlyBracket,
                '}' => Self::RCurlyBracket,
                '[' => Self::LSquareBracket,
                ']' => Self::RSquareBracket,
                ';' => Self::Semicolon,
                ',' => Self::Comma,
                '.' => {
                    if s.match_str("..", true).is_ok() {
                        Self::ThreePeriods
                    } else {
                        Self::Period
                    }
                },
                '@' => Self::At,
                ':' => {
                    s.match_str(":", true)?;
                    Self::TwoColons
                },
                v => return Err(LexingError::new(LexErrorType::SyntaxError(format!("Unknown separator {}", v)), (s.position, s.position)))
            })
        } else {
            Err(LexingError::new(LexErrorType::SyntaxError("Unknown separator".to_string()), (s.position, s.position)))
        }
    }
}
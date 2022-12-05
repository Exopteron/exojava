use self::{unicode::UnicodeInputCharacter, lines::{LineTerminator, InputCharacter}};

use super::super::{LexErrorType, LexResult, LexingError};

pub mod unicode;
pub mod lines;

pub trait BaseToken: Sized {
    fn parse(s: &mut CharStream) -> LexResult<Self>;
}


pub struct CharStream {
    chars: Vec<char>,
    position: usize,
}

impl CharStream {
    pub fn new(s: String) -> LexResult<Self> {
        let mut v = Self {
            chars: s.chars().collect(),
            position: 0,
        };

        let mut new_chars = vec![];

        while !v.is_finished() {
            let char = UnicodeInputCharacter::parse(&mut v)?;
            new_chars.push(char.0);
        }

        Ok(Self {
            chars: new_chars,
            position: 0,
        })
    }
    pub fn is_finished(&self) -> bool {
        self.position >= self.chars.len()
    }

    pub fn lookahead(&mut self) -> LexResult<char> {
        let v = self.chars.get(self.position).copied();
        v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))
    }

    pub fn next(&mut self) -> LexResult<char> {
        let v = self.chars.get(self.position).copied();
        self.position += 1;
        v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))
    }

    pub fn match_str(&mut self, str: &str) -> LexResult<()> {
        let mut cursor = 0;
        for c in str.chars() {
            let v = self.chars[self.position + cursor];
            if v != c {
                return Err(LexingError::new(
                    LexErrorType::WrongChar(v, c),
                    (cursor, cursor),
                ));
            }
            cursor += 1;
        }
        for _ in 0..cursor {
            self.next()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FinalTerminalElement {
    LineTerminator(LineTerminator),
    InputCharacter(InputCharacter)   
}

impl BaseToken for FinalTerminalElement {
    fn parse(s: &mut CharStream) -> LexResult<Self> {
        if s.lookahead()? == '\r' || s.lookahead()? == '\n' {
            Ok(Self::LineTerminator(LineTerminator::parse(s)?))
        } else {
            Ok(Self::InputCharacter(InputCharacter::parse(s)?))
        }
    }
}
use crate::lexer::{LexingError, LexErrorType, LexResult};

use super::{early::{FinalTerminalElement, lines::{InputCharacter, LineTerminator}, CharStream, BaseToken}, Tokenizable, Token};

pub struct JavaTerminalStream {
    chars: Vec<FinalTerminalElement>,
    pub position: usize,
}

impl JavaTerminalStream {
    pub fn new(mut s: CharStream) -> LexResult<Self> {
        let mut new_chars = vec![];

        while !s.is_finished() {
            new_chars.push(FinalTerminalElement::parse(&mut s)?);
        }

        Ok(Self {
            chars: new_chars,
            position: 0,
        })
    }
    pub fn is_finished(&self) -> bool {
        self.position >= self.chars.len()
    }

    pub fn get<T: Tokenizable>(&mut self) -> LexResult<Token<T>> {
        let start = self.position;
        let v = T::parse(self)?;
        let end = self.position;
        Ok(Token::new(v, (start, end)))
    }

    pub fn lookahead(&mut self) -> LexResult<FinalTerminalElement> {
        let v = self.chars.get(self.position).copied();
        v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))
    }

    pub fn next(&mut self) -> LexResult<FinalTerminalElement> {
        let v = self.chars.get(self.position).copied();
        self.position += 1;
        v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))
    }

    pub fn match_input_character(&mut self) -> LexResult<InputCharacter> {
        let v = self.chars.get(self.position).copied();
        self.position += 1;
        let c = v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))?;
        match c {
            FinalTerminalElement::InputCharacter(v) => Ok(v),
            FinalTerminalElement::LineTerminator(_) => Err(LexingError::new(
                LexErrorType::SyntaxError(
                    "Got line terminator when looking for input character".to_string(),
                ),
                (self.position, self.position),
            )),
        }
    }

    pub fn match_line_terminator(&mut self) -> LexResult<LineTerminator> {
        let v = self.chars.get(self.position).copied();
        self.position += 1;
        let c = v.ok_or(LexingError::new(
            LexErrorType::EOI,
            (self.position, self.position),
        ))?;
        match c {
            FinalTerminalElement::LineTerminator(v) => Ok(v),
            FinalTerminalElement::InputCharacter(_) => Err(LexingError::new(
                LexErrorType::SyntaxError(
                    "Got input character when looking for line terminator".to_string(),
                ),
                (self.position, self.position),
            )),
        }
    }

    pub fn match_one_of(&mut self, strs: &[&str], eat: bool) -> LexResult<usize> {
        for (idx, s) in strs.iter().enumerate() {
            if self.match_str(s, eat).is_ok() {
                return Ok(idx);
            }
        }
        return Err(LexingError::new(LexErrorType::SyntaxError(format!("Could not match one of {:?}", strs)), (self.position, self.position)))
    }

    pub fn match_str(&mut self, str: &str, eat: bool) -> LexResult<()> {
        let mut cursor = 0;
        for c in str.chars() {
            let v = self.chars[self.position + cursor];
            match v {
                FinalTerminalElement::LineTerminator(_) => {
                    return Err(LexingError::new(
                        LexErrorType::WrongChar('\n', c),
                        (cursor, cursor),
                    ));
                }
                FinalTerminalElement::InputCharacter(InputCharacter(v)) => {
                    if v != c {
                        return Err(LexingError::new(
                            LexErrorType::WrongChar(v, c),
                            (cursor, cursor),
                        ));
                    }
                }
            }
            cursor += 1;
        }
        if eat {
            for _ in 0..cursor {
                self.next()?;
            }
        }
        Ok(())
    }
}

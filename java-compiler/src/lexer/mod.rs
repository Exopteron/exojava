use std::{fmt::Display, num::ParseIntError};

pub mod tokens;
use tokens::early::BaseToken;
use thiserror::Error;

use self::tokens::early::unicode::UnicodeInputCharacter;


#[derive(Error, Debug)]
pub enum LexErrorType {
    #[error("wrong character, got {0} but expected {1}")]
    WrongChar(char, char),
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("{0}")]
    ParseIntError(ParseIntError),
    #[error("Invalid code point: {0}")]
    InvalidCodePoint(u32),
    #[error("EOI reached")]
    EOI,
}

#[derive(Error, Debug)]
pub struct LexingError {
    pub ty: LexErrorType,
    pub span: (usize, usize),
}
impl LexingError {
    pub fn new(ty: LexErrorType, span: (usize, usize)) -> Self {
        Self { ty, span }
    }
}

impl Display for LexingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty)
    }
}

pub type LexResult<T> = std::result::Result<T, LexingError>;
#[cfg(test)]
mod tests {
    use crate::lexer::tokens::{stream::JavaTerminalStream, early::CharStream, Input, Tokenizable};


    #[test]
    fn testepic() {
        let mut chars = JavaTerminalStream::new(CharStream::new(r#"
        package among.us.spaceship;

        import java.lang.UnsupportedOperationException;


        public class Imposter extends Crewmate {

            public boolean SUSSY = true;

            @Override
            public void doTasks() {
                throw new UnsupportedOperationException("I am an imposter!");
            }

        }
        
        "#.to_string()).unwrap()).unwrap();
        let i = Input::parse(&mut chars).unwrap();
        println!("{:#?}", i);
        // while !chars.is_finished() {
        //     println!("{:?}", chars.next().unwrap());
        // }
        println!("VA: {}", '⠀'.is_whitespace());
    }
}
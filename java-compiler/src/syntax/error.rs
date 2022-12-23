use std::fmt::Display;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum ParseErrorType {
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("EOI reached")]
    EOI,
}

#[derive(Error, Debug)]
pub struct ParseError {
    pub ty: ParseErrorType,
    pub span: (usize, usize),
}
impl ParseError {
    pub fn new(ty: ParseErrorType, span: (usize, usize)) -> Self {
        Self { ty, span }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty)
    }
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;
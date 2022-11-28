use std::{error::Error, fmt::{Display, Debug}};


pub type Result<T> = std::result::Result<T, ParsingError>;
impl Error for ParsingError {}

#[derive(Debug)]
pub struct ParsingError {
    pub error_type: ParsingErrorType,
    pub while_parsing: Vec<String>,
    pub position: usize
}
impl ParsingError {
    pub fn new(error_type: ParsingErrorType, while_parsing: Vec<String>, position: usize) -> Self {
        Self { error_type, while_parsing, position }
    }
}
impl Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParsingError({}, position({}))", self.error_type, self.position)
    }
}

#[derive(Debug)]
pub enum ParsingErrorType {
    EOSError,
    GenericError(Box<dyn Debug>),
    TokenizerError(String),
}
impl ParsingErrorType {
    pub fn to(self, lexer: &crate::LexerStream) -> ParsingError {
        ParsingError::new(self, lexer.lexer().currently_parsing_list(), lexer.position)
    }
}

impl Display for ParsingErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOSError => {
                write!(f, "parsing error: end of stream")
            }
            Self::GenericError(e) => write!(f, "{:?}", e),
            Self::TokenizerError(detail) => {
                write!(f, "parsing error: {}", detail)
            }
        }
    }
}

#[macro_export]
macro_rules! parse_err {
    ($position:expr, $details:expr) => {
        ParsingErrorType::TokenizerError($details.to_string()).to(&$position)
    };
}
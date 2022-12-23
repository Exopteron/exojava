use std::ops::Deref;

use self::{early::{lines::{InputCharacter, LineTerminator}, BaseToken, CharStream, FinalTerminalElement}, stream::JavaTerminalStream, whitespace::Whitespace, comment::Comment, separators::Separator, operators::Operator, keywords::Keyword, identifier::Identifier, literal::Literal};

use super::{LexErrorType, LexResult, LexingError};

pub mod comment;
pub mod early;
pub mod whitespace;
pub mod stream;
pub mod separators;
pub mod operators;
pub mod keywords;
pub mod identifier;
pub mod literal;

pub trait Tokenizable: Sized {
    fn parse(s: &mut JavaTerminalStream) -> LexResult<Self>;
}
#[derive(Clone, Copy, Debug)]
pub struct Token<T: Tokenizable> {
    pub v: T,
    pub span: (usize, usize)
}
impl<T: Tokenizable> Token<T> {
    pub fn new(v: T, span: (usize, usize)) -> Self {
        Self {
            v, span
        }
    }
}
impl<T: Tokenizable> Deref for Token<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}



#[derive(Clone, Debug)]
pub struct Input {
    pub elements: Vec<Token<JToken>>
}

impl Tokenizable for Input {
    fn parse(s: &mut JavaTerminalStream) -> LexResult<Self> {
        let mut elements = vec![];
        while !s.is_finished() {
            if Sub::parse(s).is_ok() {
                break;
            }
            let element = s.get::<InputElement>()?;
            if let InputElement::Token(v) = element.v {
                elements.push(v);
            }
        }
        Ok(Self {
            elements
        })
    }
}


#[derive(Clone, Debug)]
pub enum InputElement {
    Whitespace(),
    Comment(Token<Comment>),
    Token(Token<JToken>)
}
impl Tokenizable for InputElement {
    fn parse(s: &mut JavaTerminalStream) -> LexResult<Self> {
        if s.match_str("/", false).is_ok() {
            Ok(Self::Comment(s.get()?))
        } else if match s.lookahead()? {
            FinalTerminalElement::LineTerminator(_) => true,
            FinalTerminalElement::InputCharacter(v) => v.0.is_whitespace(),
        } {
            let w = s.get::<Whitespace>()?;
            Ok(Self::Whitespace())
        } else {
            Ok(Self::Token(s.get()?))
        }
    }
}

#[derive(Clone, Debug)]
pub enum JToken {
    Separator(Token<Separator>),
    Operator(Token<Operator>),
    Keyword(Token<Keyword>),
    Identifier(Token<Identifier>),
    Literal(Token<Literal>)
}

impl Tokenizable for JToken {
    fn parse(s: &mut JavaTerminalStream) -> LexResult<Self> {
        if Separator::lookahead_separator(s) {
            Ok(Self::Separator(s.get()?))
        } else if Operator::lookahead_operator(s) {
            Ok(Self::Operator(s.get()?))
        } else if let Ok(v) = s.get::<Keyword>() {
            Ok(Self::Keyword(v))
        } else if let Ok(v) = s.get::<Literal>() {
            Ok(Self::Literal(v))
        } else {
            Ok(Self::Identifier(s.get()?))
        }
    }
}



#[derive(Clone, Copy, Debug)]
pub struct Sub;

impl Tokenizable for Sub {
    fn parse(s: &mut JavaTerminalStream) -> LexResult<Self> {
        s.match_str("\u{001A}", true)?;
        Ok(Self)
    }
}
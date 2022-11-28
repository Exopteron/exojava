use std::fmt::Debug;

use exo_parser::{
    error::{ParsingError, ParsingErrorType},
    multi_choice,
    tokenimpl::Char,
    Parseable,
};

use super::{field::FieldDescriptor, BANNED_IDENT_CHARS};

/// Section of a class name.
#[derive(Debug)]
pub struct ClassNameSection(pub String);

impl Parseable for ClassNameSection {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        let mut str = String::new();
        while let Ok(c) = s.char() {
            if c.is_whitespace() {
                break;
            }
            if BANNED_IDENT_CHARS.contains(&c) {
                break;
            }
            if c == '$' {
                s.position -= 1;
                break;
            }
            str.push(c);
        }
        Ok(Self(str))
    }
}

/// A class name.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ClassName {
    /// The package of this class.
    pub package: Vec<String>,

    /// This class's name.
    pub class_name: String,

    /// Inner class, if any.
    pub inner_class: Option<Box<ClassName>>,
}

impl Parseable for ClassName {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        println!("Values: {:?}", s.chars());
        let mut list = vec![];
        let mut inner_class = None;
        loop {
            list.push(s.token::<ClassNameSection>()?.token.0);
            if s.token::<Char<'$'>>().is_ok() {
                inner_class = Some(Box::new(s.token::<Self>()?.token));
                break;
            }
            if list.last().unwrap().is_empty() {
                list.pop();
                break;
            }
        }
        let class_name = list
            .pop()
            .ok_or(ParsingErrorType::GenericError(Box::new("")).to(s))?;
        Ok(Self {
            class_name,
            package: list,
            inner_class,
        })
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassRefName {
    Class(ClassName),
    Array(FieldDescriptor),
}

impl Parseable for ClassRefName {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        multi_choice! {
            (ClassName)(v) => {
                return Ok(Self::Class(v.token));
            },
            (FieldDescriptor)(v) => {
                return Ok(Self::Array(v.token));
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use exo_parser::Lexer;

//     use super::ClassName;

//     #[test]
//     fn epictest() {
//         let s = Lexer::new();

//         let mut stream = Lexer::stream(s, "com/exopteron/Exo$Balls1".to_string());
//         let cln = stream.token::<ClassName>().unwrap();
//         panic!("CLN {:?}", cln);
//     }
// }

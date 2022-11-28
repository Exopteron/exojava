use exo_parser::{tokenimpl::Char, Token, Parseable, multi_choice, parse_err, error::ParsingErrorType};

use super::{field::FieldType, UnqualifiedName};



pub type VoidDescriptor = Char<'V'>;


pub type ParameterDescriptor = FieldType;

/// Return descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReturnDescriptor {
    Field(FieldType),
    Void(VoidDescriptor)
}

impl Parseable for ReturnDescriptor {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        multi_choice! {
            (FieldType)(v) => {
                return Ok(Self::Field(v.token));
            },
            (VoidDescriptor)(v) => {
                return Ok(Self::Void(v.token));
            }
        }
    }
}

/// A method descriptor contains zero or more 
/// parameter descriptors, representing the types 
/// of parameters that the method takes, and a 
/// return descriptor, representing the type of 
/// the value (if any) that the method returns. 
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodDescriptor {
    pub parameters: Vec<ParameterDescriptor>,
    pub return_desc: ReturnDescriptor
}

impl Parseable for MethodDescriptor {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        let mut par = exo_parser::enclosed::<Char<'('>, Char<')'>>(s)?;
        let mut params = vec![];
        while !par.ended() {
            params.push(par.token::<ParameterDescriptor>()?.token);
        }
        Ok(Self {
            parameters: params,
            return_desc: s.token()?.token
        })
    }
}

/// Method name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MethodName {
    Clinit,
    Init,
    Generic(UnqualifiedName)
}


impl Parseable for MethodName {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        if s.token::<Char<'<'>>().is_ok() {
            s.position -= 1;
            let mut enc = exo_parser::enclosed::<Char<'<'>, Char<'>'>>(s)?;
            let name = enc.token::<UnqualifiedName>()?;
            match name.0.as_str() {
                "clinit" => {
                    Ok(Self::Clinit)
                },
                "init" => {
                    Ok(Self::Init)
                }
                _ => Err(parse_err!(s, "bad method name"))
            }
        } else {
            Ok(Self::Generic(s.token()?.token))
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use exo_parser::Lexer;

//     use crate::item::ids::{field::{BaseType, FieldDescriptor}, method::MethodDescriptor};

//     #[test]
//     fn swagger() {
//         let s = Lexer::new();

//         let mut stream = Lexer::stream(s, "(IDLjava/lang/Thread;)Ljava/lang/Object;".to_string());
//         let cln = stream.token::<MethodDescriptor>().unwrap();
//         panic!("CLN {:#?}", cln);
//     }
// }

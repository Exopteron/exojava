use exo_parser::{multi_choice, tokenimpl::Char, Parseable, Token};

use super::class::ClassName;

/// Base types.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BaseType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
}

impl Parseable for BaseType {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        multi_choice! {
            Char<'B'>(_) => {
                return Ok(Self::Byte);
            },
            Char<'C'>(_) => {
                return Ok(Self::Char);
            },
            Char<'D'>(_) => {
                return Ok(Self::Double);
            },
            Char<'F'>(_) => {
                return Ok(Self::Float);
            },
            Char<'I'>(_) => {
                return Ok(Self::Int);
            },
            Char<'J'>(_) => {
                return Ok(Self::Long);
            },
            Char<'S'>(_) => {
                return Ok(Self::Short);
            },
            Char<'Z'>(_) => {
                return Ok(Self::Boolean);
            }
        }
    }
}
/// Object type.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectType {
    pub class_name: ClassName
}

impl Parseable for ObjectType {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        let mut par = exo_parser::enclosed::<Char<'L'>, Char<';'>>(s)?;
        Ok(Self {
            class_name: par.token::<ClassName>()?.token
        })
    }
}


/// Array type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayType(pub ComponentType);

impl Parseable for ArrayType {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        s.token::<Char<'['>>()?;
        Ok(Self(Box::new(s.token()?.token)))
    }
}


pub type ComponentType = Box<FieldType>;


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Field type.
pub enum FieldType {
    BaseType(BaseType),
    ObjectType(ObjectType),
    ArrayType(ArrayType)
}

impl Parseable for FieldType {
    fn parse(s: &mut exo_parser::LexerStream) -> exo_parser::error::Result<Self> {
        multi_choice! {
            (BaseType)(v) => {
                return Ok(Self::BaseType(v.token));
            },
            (ObjectType)(v) => {
                return Ok(Self::ObjectType(v.token));
            },
            (ArrayType)(v) => {
                return Ok(Self::ArrayType(v.token));
            }
        }
    }
}


/// A field descriptor represents the type of a class, instance, or local variable. 
pub type FieldDescriptor = FieldType;



// #[cfg(test)]
// mod tests {
//     use exo_parser::Lexer;

//     use crate::item::ids::field::{BaseType, FieldDescriptor};

//     #[test]
//     fn swag() {
//         let s = Lexer::new();

//         let mut stream = Lexer::stream(s, "[Lcom/exopteron/Balls$Cool;".to_string());
//         let cln = stream.token::<FieldDescriptor>().unwrap();
//         panic!("CLN {:#?}", cln);
//     }
// }

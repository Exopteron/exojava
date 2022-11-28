use std::string::FromUtf8Error;


/// An error which can occur on deserialization of a class file.
#[derive(Debug)]
pub enum ClassFileError {
    /// A generic I/O error.
    IoError(std::io::Error),

    /// Returned when a class file has a bad magic number.
    BadMagicNumber(u32),

    /// Returned when an unknown constant pool tag is found.
    UnknownConstantPoolTag(u8),

    /// Returned when invalid UTF-8 is found.
    InvalidUTF8Error(FromUtf8Error),

    /// Returned when an unknown reference kind is found.
    UnknownReferenceKind(u8),

    /// Returned when bad class access flags are found.
    BadClassAccessFlags,

    /// Returned when an unknown verification type info tag is found.
    UnknownVerificationTypeInfo,
    
    /// Returned when an unknown stack map frame tag is found. 
    UnknownStackMapFrameTag(u8),

    /// Returned when an unknown element value type is found.
    UnknownElementValueType(char),

    /// Returned when an unknown target type value is found.
    UnknownTargetTypeValue(u8),

    /// Returned when an unknown type path kind value is found.
    UnknownTypePathKind(u8),
    
    /// Returned when bad formal parameter access flags are found.
    BadFormalParameterAccessFlags,

    /// Returned when a string constant was expected.
    ExpectedString,

    /// Returned when an unknown attribute is found.
    UnknownAttribute(String),

    /// Returned when bad field access flags are found.
    BadFieldAccessFlags,

    /// Returned when bad method access flags are found.
    BadMethodAccessFlags,

    /// Returned when an unknown opcode is found.
    UnknownOpcodeError(u8),

    /// Returned when an unknown enum variant is found.
    UnknownEnumVariant(&'static str, i32)
}

pub type Result<T> = std::result::Result<T, ClassFileError>;

use std::io::Read;

use crate::{
    error::{self, ClassFileError},
    stream::ClassFileStream,
};

pub use self::refkind::RefKind;

use crate::item::ClassFileItem;

use super::ConstantPool;

/// The tag values for each type of constant pool entry.
mod tags {
    pub const CONSTANT_Class: u8 = 7;
    pub const CONSTANT_Fieldref: u8 = 9;
    pub const CONSTANT_Methodref: u8 = 10;
    pub const CONSTANT_InterfaceMethodref: u8 = 11;
    pub const CONSTANT_String: u8 = 8;
    pub const CONSTANT_Integer: u8 = 3;
    pub const CONSTANT_Float: u8 = 4;
    pub const CONSTANT_Long: u8 = 5;
    pub const CONSTANT_Double: u8 = 6;
    pub const CONSTANT_NameAndType: u8 = 12;
    pub const CONSTANT_Utf8: u8 = 1;
    pub const CONSTANT_MethodHandle: u8 = 15;
    pub const CONSTANT_MethodType: u8 = 16;
    pub const CONSTANT_InvokeDynamic: u8 = 18;
}

/// The possible reference kind values for method handles.
mod refkind {
    use crate::error::{self, ClassFileError};

    pub const REF_getField: u8 = 1;
    pub const REF_getStatic: u8 = 2;
    pub const REF_putField: u8 = 3;
    pub const REF_putStatic: u8 = 4;
    pub const REF_invokeVirtual: u8 = 5;
    pub const REF_invokeStatic: u8 = 6;
    pub const REF_invokeSpecial: u8 = 7;
    pub const REF_newInvokeSpecial: u8 = 8;
    pub const REF_invokeInterface: u8 = 9;
    #[derive(Debug, Clone, Copy)]
    pub enum RefKind {
        REF_getField = REF_getField as isize,
        REF_getStatic = REF_getStatic as isize,
        REF_putField = REF_putField as isize,
        REF_putStatic = REF_putStatic as isize,
        REF_invokeVirtual = REF_invokeVirtual as isize,
        REF_invokeStatic = REF_invokeStatic as isize,
        REF_invokeSpecial = REF_invokeSpecial as isize,
        REF_newInvokeSpecial = REF_newInvokeSpecial as isize,
        REF_invokeInterface = REF_invokeInterface as isize,
    }

    impl RefKind {
        pub fn decode(v: u8) -> error::Result<Self> {
            match v {
                REF_getField => Ok(Self::REF_getField),
                REF_getStatic => Ok(Self::REF_getStatic),
                REF_putField => Ok(Self::REF_putField),
                REF_putStatic => Ok(Self::REF_putStatic),
                REF_invokeVirtual => Ok(Self::REF_invokeVirtual),
                REF_invokeStatic => Ok(Self::REF_invokeStatic),
                REF_invokeSpecial => Ok(Self::REF_invokeSpecial),
                REF_newInvokeSpecial => Ok(Self::REF_newInvokeSpecial),
                REF_invokeInterface => Ok(Self::REF_invokeInterface),
                _ => Err(ClassFileError::UnknownReferenceKind(v)),
            }
        }
    }
}


/// A constant pool entry.
/// 
/// Deserialization does not perform any index verification.
#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    /// The CONSTANT_Class_info structure is used to represent a class or an interface.
    Class {
        /// The value of the name_index item must be a valid index into the constant_pool table. The constant_pool entry at that index must
        /// be a CONSTANT_Utf8_info structure representing a valid binary class or interface name encoded in internal form.
        name_index: u16,
    },
    Fieldref {
        /// The value of the class_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Class_info structure
        /// representing a class or interface type that has the field or method as a member.
        ///
        /// The class_index item may be either a class type or an interface type.
        class_index: u16,
        /// The value of the name_and_type_index item must be a valid index
        /// into the constant_pool table. The constant_pool entry at that index
        /// must be a CONSTANT_NameAndType_info structure. This constant_pool entry
        /// indicates the name and descriptor of the field or method.
        ///
        ///
        /// In a CONSTANT_Fieldref_info, the indicated descriptor must be a field descriptor.
        /// Otherwise, the indicated descriptor must be a method descriptor.
        name_and_type_index: u16,
    },
    Methodref {
        /// The value of the class_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Class_info structure
        /// representing a class or interface type that has the field or method as a member.
        ///
        /// The class_index item must be a class type, not an interface type.
        class_index: u16,
        /// The value of the name_and_type_index item must be a valid index
        /// into the constant_pool table. The constant_pool entry at that index
        /// must be a CONSTANT_NameAndType_info structure. This constant_pool entry
        /// indicates the name and descriptor of the field or method.
        ///
        /// If the name of the method of a CONSTANT_Methodref_info structure begins with a '<' ('\u003c'),
        /// then the name must be the special name <init>, representing an instance initialization method.
        /// The return type of such a method must be void.
        name_and_type_index: u16,
    },
    InterfaceMethodref {
        /// The value of the class_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Class_info structure
        /// representing a class or interface type that has the field or method as a member.
        ///
        /// The class_index item must be an interface type.
        class_index: u16,
        /// The value of the name_and_type_index item must be a valid index
        /// into the constant_pool table. The constant_pool entry at that index
        /// must be a CONSTANT_NameAndType_info structure. This constant_pool entry
        /// indicates the name and descriptor of the field or method.
        name_and_type_index: u16,
    },
    /// The CONSTANT_String_info structure is used to represent constant objects of the type String.
    String {
        /// The value of the string_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Utf8_info structure representing
        /// the sequence of Unicode code points to which the String object is to be initialized.
        string_index: u16,
    },
    /// The CONSTANT_Integer_info represents a 4-byte numeric int constant:
    Integer {
        /// The bytes item of the CONSTANT_Integer_info structure represents the value of the int constant.
        /// The bytes of the value are stored in big-endian (high byte first) order.
        bytes: i32,
    },
    /// The CONSTANT_Float_info represents a 4-byte numeric float constant:
    Float {
        /// The bytes item of the CONSTANT_Float_info structure represents the value of the float constant
        /// in IEEE 754 floating-point single format. The bytes of the single format representation are
        /// stored in big-endian (high byte first) order.
        float: u32,
    },
    /// The CONSTANT_Long_info represents an 8-byte numeric long constant:
    Long {
        /// The unsigned high_bytes and low_bytes items of the CONSTANT_Long_info structure together
        /// represent the value of the long constant.
        ///
        /// We combine them to a single i64 at parsing stage.
        bytes: i64,
    },
    /// The CONSTANT_Double_info represents an 8-byte numeric double constant:
    Double {
        /// The high_bytes and low_bytes items of the CONSTANT_Double_info structure together
        /// represent the double value in IEEE 754 floating-point double format. The bytes
        /// of each item are stored in big-endian (high byte first) order.
        bytes: u64,
    },
    /// The CONSTANT_NameAndType_info structure is used to represent a field or method, without indicating which class or interface type it belongs to.
    NameAndType {
        /// The value of the name_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Utf8_info structure
        /// representing either the special method name <init> or a valid unqualified
        /// name denoting a field or method.
        name_index: u16,
        /// The value of the descriptor_index item must be a valid index into the constant_pool table.
        /// The constant_pool entry at that index must be a CONSTANT_Utf8_info structure representing
        /// a valid field descriptor or method descriptor.
        descriptor_index: u16,
    },
    /// The CONSTANT_Utf8_info structure is used to represent constant string values.
    Utf8 { data: String },
    /// The CONSTANT_MethodHandle_info structure is used to represent a method handle.
    MethodHandle {
        /// The value of the reference_kind item must be in the range 1 to 9.
        /// The value denotes the kind of this method handle, which
        /// characterizes its bytecode behavior.
        reference_kind: RefKind,

        /// The value of the reference_index item must be a valid index into the constant_pool table. The constant_pool entry at that index must be as follows:

        /// If the value of the reference_kind item is 1 (REF_getField), 2 (REF_getStatic), 3 (REF_putField), or 4 (REF_putStatic), then the constant_pool entry at that index must be a CONSTANT_Fieldref_info (§4.4.2) structure representing a field for which a method handle is to be created.

        /// If the value of the reference_kind item is 5 (REF_invokeVirtual) or 8 (REF_newInvokeSpecial), then the constant_pool entry at that index must be a CONSTANT_Methodref_info structure (§4.4.2) representing a class's method or constructor (§2.9) for which a method handle is to be created.

        /// If the value of the reference_kind item is 6 (REF_invokeStatic) or 7 (REF_invokeSpecial), then if the class file version number is less than 52.0, the constant_pool entry at that index must be a CONSTANT_Methodref_info structure representing a class's method for which a method handle is to be created; if the class file version number is 52.0 or above, the constant_pool entry at that index must be either a CONSTANT_Methodref_info structure or a CONSTANT_InterfaceMethodref_info structure (§4.4.2) representing a class's or interface's method for which a method handle is to be created.

        /// If the value of the reference_kind item is 9 (REF_invokeInterface), then the constant_pool entry at that index must be a CONSTANT_InterfaceMethodref_info structure representing an interface's method for which a method handle is to be created.

        /// If the value of the reference_kind item is 5 (REF_invokeVirtual), 6 (REF_invokeStatic), 7 (REF_invokeSpecial), or 9 (REF_invokeInterface), the name of the method represented by a CONSTANT_Methodref_info structure or a CONSTANT_InterfaceMethodref_info structure must not be <init> or <clinit>.

        /// If the value is 8 (REF_newInvokeSpecial), the name of the method represented by a CONSTANT_Methodref_info structure must be <init>.
        reference_index: u16,
    },
    /// The CONSTANT_MethodType_info structure is used to represent a method type.
    MethodType {
        /// The value of the descriptor_index item must be a valid index into the
        /// constant_pool table. The constant_pool entry at that index must be
        /// a CONSTANT_Utf8_info structure representing a method descriptor.
        descriptor_index: u16,
    },
    /// The CONSTANT_InvokeDynamic_info structure is used by an invokedynamic instruction
    /// (§invokedynamic) to specify a bootstrap method, the dynamic invocation name,
    /// the argument and return types of the call, and optionally, a sequence of additional
    /// constants called static arguments to the bootstrap method.
    InvokeDynamic {
        /// The value of the bootstrap_method_attr_index item must be a
        /// valid index into the bootstrap_methods array of the
        /// bootstrap method table of this class file.
        bootstrap_method_attr_index: u16,
        /// The value of the name_and_type_index item must be a valid
        /// index into the constant_pool table. The constant_pool entry
        /// at that index must be a CONSTANT_NameAndType_info structure
        /// representing a method name and method descriptor.
        name_and_type_index: u16,
    },
}

impl ClassFileItem for ConstantPoolEntry {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: Sized,
    {
        match s.read_u1()? {
            tags::CONSTANT_Class => Ok(Self::Class {
                name_index: s.read_u2()?,
            }),
            tags::CONSTANT_Fieldref => Ok(Self::Fieldref {
                class_index: s.read_u2()?,
                name_and_type_index: s.read_u2()?,
            }),
            tags::CONSTANT_Methodref => Ok(Self::Methodref {
                class_index: s.read_u2()?,
                name_and_type_index: s.read_u2()?,
            }),
            tags::CONSTANT_InterfaceMethodref => Ok(Self::InterfaceMethodref {
                class_index: s.read_u2()?,
                name_and_type_index: s.read_u2()?,
            }),
            tags::CONSTANT_String => Ok(Self::String {
                string_index: s.read_u2()?,
            }),
            tags::CONSTANT_Integer => Ok(Self::Integer {
                bytes: s.read_u4()? as i32,
            }),
            tags::CONSTANT_Float => Ok(Self::Float {
                float: s.read_u4()?,
            }),
            tags::CONSTANT_Long => Ok(Self::Long {
                bytes: i64::from_be_bytes(s.read::<8>()?),
            }),
            tags::CONSTANT_Double => Ok(Self::Double {
                bytes: u64::from_be_bytes(s.read::<8>()?),
            }),
            tags::CONSTANT_NameAndType => Ok(Self::NameAndType {
                name_index: s.read_u2()?,
                descriptor_index: s.read_u2()?,
            }),
            tags::CONSTANT_Utf8 => {
                let length = s.read_u2()?;
                let bytes = s.read_dynamic(length as usize)?;
                Ok(Self::Utf8 {
                    data: String::from_utf8(bytes).map_err(ClassFileError::InvalidUTF8Error)?,
                })
            }
            tags::CONSTANT_MethodHandle => Ok(Self::MethodHandle {
                reference_kind: RefKind::decode(s.read_u1()?)?,
                reference_index: s.read_u2()?,
            }),
            tags::CONSTANT_MethodType => Ok(Self::MethodType {
                descriptor_index: s.read_u2()?,
            }),
            tags::CONSTANT_InvokeDynamic => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: s.read_u2()?,
                name_and_type_index: s.read_u2()?,
            }),
            v => Err(ClassFileError::UnknownConstantPoolTag(v)),
        }
    }
}

// /// Creates a string from the class file format's
// /// modified UTF-8 encoding.
// fn class_utf8(b: &[u8]) -> Option<String> {
//     let mut c = vec![];
//     let mut index = 0;

//     while (index < b.len()) {
//         let mut byte = b[index];
//         index += 1;
//         let mut k = byte.leading_ones();
//         let mask = (1 << (8 - k)) - 1;
//         let mut value = byte & mask;
//         while k > 0 && byte
//     }
//     todo!()
// }

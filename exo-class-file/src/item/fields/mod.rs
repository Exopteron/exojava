use std::io::Read;

use crate::{
    error::{self, ClassFileError},
    stream::ClassFileStream,
};

use super::{attribute_info::{Attributes, AttributesCollection}, ClassFileItem, ConstantPool};

/// Field info.
#[derive(Debug)]
pub struct FieldInfo {
    /// The value of the access_flags item is a mask of
    /// flags used to denote access permission
    /// to and properties of this field.
    pub access_flags: FieldAccessFlags,
    /// The value of the name_index item must be a
    /// valid index into the constant_pool table.
    ///
    /// The constant_pool entry at that index must
    /// be a CONSTANT_Utf8_info structure (§4.4.7)
    /// which represents a valid unqualified
    /// name denoting a field (§4.2.2).
    pub name_index: u16,
    /// The value of the descriptor_index item
    /// must be a valid index into the constant_pool
    /// table. The constant_pool entry at that index
    /// must be a CONSTANT_Utf8_info structure (§4.4.7)
    /// which represents a valid field descriptor (§4.3.2).
    pub descriptor_index: u16,
    /// Each value of the attributes table must be an attribute_info structure (§4.7).
    /// A field can have any number of optional attributes associated with it.
    pub attributes: AttributesCollection,
}

impl ClassFileItem for FieldInfo {
    fn read_from_stream<R: Read>(
        s: &mut ClassFileStream<R>,
        cp: Option<&ConstantPool>,
    ) -> error::Result<Self>
    where
        Self: Sized,
    {
        let access_flags =
            FieldAccessFlags::from_bits(s.read_u2()?).ok_or(ClassFileError::BadFieldAccessFlags)?;

        let name_index = s.read_u2()?;

        let descriptor_index = s.read_u2()?;

        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes: AttributesCollection::read_from_stream(s, cp)?,
        })
    }
}

bitflags::bitflags! {
    pub struct FieldAccessFlags: u16 {
        /// Declared public; may be accessed from outside its package.
        const ACC_PUBLIC = 0x0001;
        /// Declared private; usable only within the defining class.
        const ACC_PRIVATE = 0x0002;
        /// Declared protected; may be accessed within subclasses.
        const ACC_PROTECTED = 0x0004;
        /// Declared static.
        const ACC_STATIC = 0x0008;
        /// Declared final; never directly assigned to after object construction (JLS §17.5).
        const ACC_FINAL = 0x0010;
        /// Declared volatile; cannot be cached.
        const ACC_VOLATILE = 0x0040;
        /// Declared transient; not written or read by a persistent object manager.
        const ACC_TRANSIENT = 0x0080;
        /// Declared synthetic; not present in the source code.
        const ACC_SYNTHETIC = 0x1000;
        /// Declared as an element of an enum.
        const ACC_ENUM = 0x4000;
    }
}

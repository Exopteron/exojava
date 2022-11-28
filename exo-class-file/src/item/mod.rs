use std::io::Read;

pub mod file;
pub mod constant_pool;
pub mod attribute_info;
pub mod fields;
pub mod methods;
pub mod opcodes;
pub mod ids;

use crate::{error, stream::ClassFileStream};

pub use self::constant_pool::ConstantPool;

/// A component of a class file.
pub trait ClassFileItem {
    /// Read this item from a class file stream.
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: std::marker::Sized;
}

use std::io::Read;

use crate::{
    error::{self, ClassFileError},
    stream::ClassFileStream,
};

use super::{fields::FieldInfo, methods::MethodInfo, attribute_info::{Attributes, AttributesCollection}};
pub use super::{constant_pool::ConstantPool, ClassFileItem};

/// The magic number of a class file.
pub const CLASS_MAGIC: u32 = 0xCAFEBABE;

bitflags::bitflags! {
    pub struct ClassAccessFlags: u16 {
        /// Declared public; may be accessed from outside its package.
        const ACC_PUBLIC = 0x0001;
        /// Declared final; no subclasses allowed.
        const ACC_FINAL = 0x0010;
        /// Treat superclass methods specially when invoked by the invokespecial instruction.
        const ACC_SUPER = 0x0020;
        /// Is an interface, not a class.
        const ACC_INTERFACE = 0x0200;
        /// Declared abstract; must not be instantiated.
        const ACC_ABSTRACT = 0x0400;
        /// Declared synthetic; not present in the source code.
        const ACC_SYNTHETIC = 0x1000;
        /// Declared as an annotation type.
        const ACC_ANNOTATION = 0x2000;
        /// Declared as an enum type.
        const ACC_ENUM = 0x4000;
    }
}



/// A class file.
#[derive(Debug)]
pub struct ClassFile {
    /// The class file's version (major, minor).
    pub version: (u16, u16),
    /// The constant pool.
    pub constant_pool: ConstantPool,
    /// This class's access flags.
    pub access_flags: ClassAccessFlags,
    /// The value of the this_class item must be a valid index
    /// into the constant_pool table. The constant_pool entry
    /// at that index must be a CONSTANT_Class_info structure
    /// representing the class or interface
    /// defined by this class file.
    pub this_class: u16,
    /// For a class, the value of the super_class item either
    /// must be zero or must be a valid index into the
    /// constant_pool table. If the value of the super_class
    /// item is nonzero, the constant_pool entry at that index
    /// must be a CONSTANT_Class_info structure representing
    /// the direct superclass of the class defined by this
    /// class file. Neither the direct superclass nor any
    /// of its superclasses may have the ACC_FINAL flag set
    /// in the access_flags item of its ClassFile structure.
    ///
    /// If the value of the super_class item is zero, then
    /// this class file must represent the class Object,
    /// the only class or interface without a direct superclass.

    /// For an interface, the value of the super_class item
    /// must always be a valid index into the constant_pool
    /// table. The constant_pool entry at that index must
    /// be a CONSTANT_Class_info structure representing the class Object.
    pub super_class: u16,
    /// Each value in the interfaces array must be a valid index
    /// into the constant_pool table. The constant_pool entry at
    /// each value of interfaces[i], where 0 ≤ i < interfaces_count,
    /// must be a CONSTANT_Class_info structure representing an
    /// interface that is a direct superinterface of this class
    ///  or interface type, in the left-to-right order given
    /// in the source for the type.
    pub interfaces: Vec<u16>,

    /// Each value in the fields table must be a field_info structure (§4.5) 
    /// giving a complete description of a field in this class or interface. 
    /// 
    /// The fields table includes only those fields that are declared by 
    /// this class or interface. It does not include items representing 
    /// fields that are inherited from superclasses or superinterfaces.
    pub fields: Vec<FieldInfo>,
    /// Each value in the methods table must be a method_info structure (§4.6) 
    /// giving a complete description of a method in this class or interface. 
    /// 
    /// If neither of the ACC_NATIVE and ACC_ABSTRACT flags are set in the access_flags 
    /// item of a method_info structure, the Java Virtual Machine instructions 
    /// implementing the method are also supplied.
    /// 
    /// The method_info structures represent all methods declared by this class 
    /// or interface type, including instance methods, class methods, instance 
    /// initialization methods (§2.9), and any class or interface 
    /// initialization method (§2.9). 
    /// 
    /// The methods table does not include items representing methods 
    /// that are inherited from superclasses or superinterfaces. 
    pub methods: Vec<MethodInfo>,
    /// Each value of the attributes table must be an attribute_info structure (§4.7). 
    pub attributes: AttributesCollection
}

impl ClassFileItem for ClassFile {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: Sized,
    {
        // check magic number
        let magic = s.read_u4()?;
        if magic != CLASS_MAGIC {
            return Err(ClassFileError::BadMagicNumber(magic));
        }

        // read file version
        let minor_version = s.read_u2()?;
        let major_version = s.read_u2()?;

        println!("Major: {}", major_version);

        println!("Minor: {}", minor_version);


        // read constant pool
        let constant_pool = ConstantPool::read_from_stream(s, None)?;

        // read access flags
        let access_flags =
            ClassAccessFlags::from_bits(s.read_u2()?).ok_or(ClassFileError::BadClassAccessFlags)?;

        // read this class & super class
        let this_class = s.read_u2()?;
        let super_class = s.read_u2()?;

        // read interfaces
        let interfaces_count = s.read_u2()?;
        let interfaces = s.read_sequence::<u16>(Some(&constant_pool), interfaces_count as usize)?;

        // read fields
        let fields_count = s.read_u2()?;
        let fields = s.read_sequence(Some(&constant_pool), fields_count as usize)?;

        // read methods
        let methods_count = s.read_u2()?;
        let methods = s.read_sequence(Some(&constant_pool), methods_count as usize)?;

        // read attributes
        let attributes = AttributesCollection::read_from_stream(s, Some(&constant_pool))?;

        Ok(Self {
            version: (major_version, minor_version),
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::item::ClassFileItem;

    use super::ClassFile;

    #[test]
    fn class_file_test() {
        let file = include_bytes!("../../../local/Test.class");

        let mut class_file = ClassFile::read_from_stream(&mut crate::stream::ClassFileStream::new(
            &mut Cursor::new(file),
        ), None)
        .unwrap();
        class_file.constant_pool.verify_structure(&class_file).unwrap();
        panic!("File: {:#?}", class_file);
    }
}

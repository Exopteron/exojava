use std::io::Read;

use exo_parser::Lexer;

use crate::{error::{self, ClassFileError}, stream::ClassFileStream};

pub use self::entry::{ConstantPoolEntry, RefKind};

use super::{ClassFileItem, ids::{class::ClassName, field::FieldDescriptor, method::{MethodDescriptor, ReturnDescriptor, MethodName}, UnqualifiedName}, file::ClassFile, attribute_info::{Attributes, attrtype}};

mod entry;


/// The constant pool. Contains all constant pool entries.
///
/// Does not perform index verification upon deserialization.
#[derive(Debug)]
pub struct ConstantPool {
    /// The entries of the constant pool.
    pub entries: Vec<ConstantPoolEntry>,
}

impl ClassFileItem for ConstantPool {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: Sized,
    {
        let len = (s.read_u2()? - 1) as usize;
        Ok(Self {
            entries: s.read_sequence::<ConstantPoolEntry>(cp, len)?,
        })
    }
}
#[derive(Debug)]
pub struct IndexVerificationError {
    pub index: usize,
    pub ty: IndexVerificationErrorType,
}

#[derive(Debug)]
pub enum IndexVerificationErrorType {
    /// Returned if the `name_index` within a `Class` constant pool entry
    /// is not a `UTF8` entry.
    ClassNameIndexNotUTF8,
    /// Returned if the `class_index` of an
    /// `InterfaceMethodref`, `Methodref` or `Fieldref` is not a class.
    InterfaceMethod_Field_Method_ref_ClassIndexNotClass,
    /// Returned if the `name_and_type_index` of an
    /// `InterfaceMethodref`, `Methodref` or `Fieldref` is not a `NameAndType` constant pool entry.
    InterfaceMethod_Field_Method_ref_NameAndTypeIndexNotNameAndTypeInfo,
    /// Returned if the `string_index` of a `String` constant is not
    /// a `UTF8` constant pool entry.
    StringIndexNotUTF8,
    /// Returned if the `name_index` of a `NameAndType` constant is not
    /// a `UTF8` constant pool entry.
    NameAndTypeNameIndexNotUTF8,
    /// Returned if the `descriptor_index` of a `NameAndType` constant is not
    /// a `UTF8` constant pool entry.
    NameAndTypeDescriptorIndexNotUTF8,
    /// Returned if the `reference_index` of a `MethodHandle` is invalid.
    MethodHandleReferenceIndexBadType,
    /// Returned if the `descriptor_index` of a `MethodType` is not a `UTF8` constant pool entry.
    MethodTypeDescriptorIndexNotUTF8,
    /// Returned if the `name_and_type_index` of an `InvokeDynamic` constant is not a `NameAndType` constant.
    InvokeDynamicNameAndTypeIndexNotNameAndType
}

macro_rules! verify_index {
    ($index:expr, $e:expr, $err:expr) => {
        if !$e {
            Err(IndexVerificationError {
                index: $index,
                ty: $err,
            })
        } else {
            Ok(())
        }
    };
}

/// Constant pool verification errors.
#[derive(Debug)]
pub enum ConstantPoolVerificationError {
    /// Returned with malformed types.
    IndexVerificationError(IndexVerificationError),

    /// Returned with class file errors.
    ClassFileError(ClassFileError),

    /// Returned if the class name in a class info structure is malformed.
    ClassInfoStructureMalformedClassName,

    /// Returned if the class name is malformed in a `Fieldref`, `Methodref` or `InterfaceMethodref`.
    RefInfoMalformedClassName,

    /// Returned if the field descriptor of a `Fieldref` is malformed.
    FieldRefMalformedFieldDescriptor,

    /// Returned if the method descriptor of a `Methodref` or `InterfaceMethodref` is malformed.
    MethodRefMalformedMethodDescriptor,

    /// Returned if the name of a `Methodref` is invalid.
    MethodRefInvalidName,

    /// Returned if the return type of an `<init>` method is not `void`.
    MethodRefInitReturnNotVoid,

    /// Returned when an invalid name is found in a `NameAndType` structure.
    NameAndTypeNameInvalid,

    /// Returned when the descriptor of a `NameAndType` structure is malformed.
    NameAndTypeMalformedDescriptor,

    /// Returned when the descriptor of a `MethodType` is malformed.
    MethodTypeMalformedDescriptor,

    /// Returned if the bootstrap methods index is invalid.
    InvokeDynamicInvalidBootstrapMethodsIndex,

    /// Returned if the class has no bootstrap methods attribute.
    InvokeDynamicNoBootstrapMethodsAttr,

    /// Returned if the method descriptor is invalid.
    InvokeDynamicInvalidMethodDescriptor,

    /// Returned if the method name is invalid.
    InvokeDynamicInvalidMethodName,

    /// Returned if there are more than 1 bootstrap methods attributes on a class.
    BootstrapMethodsTooMany
}

impl ConstantPool {
    /// Get a constant from the pool. Entries are based on 1.
    pub fn get_constant(&self, index: usize) -> &ConstantPoolEntry {
        &self.entries[index - 1]
    }

    /// Get a UTF-8 constant from the pool.
    pub fn get_utf8_constant(&self, index: usize) -> error::Result<&str> {
        let c = self.get_constant(index);
        if let ConstantPoolEntry::Utf8 { data } = c {
            return Ok(data);
        }
        Err(ClassFileError::ExpectedString)
    }
    
    /// Verifies that the constant pool is well-formed.
    pub fn verify_structure(&self, class_file: &ClassFile) -> std::result::Result<(), ConstantPoolVerificationError> {
        self.verify_cp_index_types().map_err(ConstantPoolVerificationError::IndexVerificationError)?;

        for entry in self.entries.iter() {
            match entry {
                ConstantPoolEntry::Class { name_index } => {
                    let name = self.get_utf8_constant(*name_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let lexer = Lexer::new();
                    let mut stream = Lexer::stream(lexer, name.to_string());
                    if stream.token::<ClassName>().is_err() && stream.token::<FieldDescriptor>().is_err() {
                        return Err(ConstantPoolVerificationError::ClassInfoStructureMalformedClassName);
                    }
                },
                ConstantPoolEntry::Methodref { class_index, name_and_type_index } | ConstantPoolEntry::Fieldref { class_index, name_and_type_index } | ConstantPoolEntry::InterfaceMethodref { class_index, name_and_type_index } => {
                    let name = match self.get_constant(*class_index as usize) {
                        ConstantPoolEntry::Class { name_index } => self.get_utf8_constant(*name_index as usize),
                        _ => panic!("we checked types")
                    }.map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let lexer = Lexer::new();
                    let mut stream = Lexer::stream(lexer.clone(), name.to_string());
                    if stream.token::<ClassName>().is_err() {
                        return Err(ConstantPoolVerificationError::RefInfoMalformedClassName);
                    }

                    let (name_index, descriptor_index) = match self.get_constant(*name_and_type_index as usize) {
                        ConstantPoolEntry::NameAndType { name_index, descriptor_index } => (name_index, descriptor_index),
                        _ => panic!("Should be impossible, we verified types")                        
                    };

                    if matches!(entry, ConstantPoolEntry::Fieldref { .. }) {
                        let descriptor = self.get_utf8_constant(*descriptor_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                        let mut stream = Lexer::stream(lexer, descriptor.to_string());  
                        if stream.token::<FieldDescriptor>().is_err() {
                            return Err(ConstantPoolVerificationError::FieldRefMalformedFieldDescriptor);
                        }
                    } else {
                        let descriptor = self.get_utf8_constant(*descriptor_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                        let mut stream = Lexer::stream(lexer, descriptor.to_string()); 
                        let d = stream.token::<MethodDescriptor>(); 
                        if d.is_err() {
                            return Err(ConstantPoolVerificationError::MethodRefMalformedMethodDescriptor);
                        }
                        let d = d.unwrap();
                        if matches!(entry, ConstantPoolEntry::Methodref { .. }) {
                            let name = self.get_utf8_constant(*name_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                            if name.starts_with('<') {
                                if name != "<init>" {
                                    return Err(ConstantPoolVerificationError::MethodRefInvalidName);
                                }
                                if !matches!(d.return_desc, ReturnDescriptor::Void(_)) {
                                    return Err(ConstantPoolVerificationError::MethodRefInitReturnNotVoid);
                                }
                            }
                        }
                    }
                },
                ConstantPoolEntry::NameAndType { name_index, descriptor_index } => {
                    let name = self.get_utf8_constant(*name_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let lexer = Lexer::new();
                    let mut stream = Lexer::stream(lexer.clone(), name.to_string());
                    if stream.token::<UnqualifiedName>().is_err() {
                        return Err(ConstantPoolVerificationError::NameAndTypeNameInvalid);
                    }
                    
                    let descriptor = self.get_utf8_constant(*descriptor_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let mut stream = Lexer::stream(lexer.clone(), descriptor.to_string());
                    if stream.token::<MethodDescriptor>().is_err() && stream.token::<FieldDescriptor>().is_err() {
                        return Err(ConstantPoolVerificationError::NameAndTypeMalformedDescriptor);
                    }
                },
                ConstantPoolEntry::MethodType { descriptor_index } => {
                    let lexer = Lexer::new();
                    let descriptor = self.get_utf8_constant(*descriptor_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let mut stream = Lexer::stream(lexer.clone(), descriptor.to_string());
                    if stream.token::<MethodDescriptor>().is_err() {
                        return Err(ConstantPoolVerificationError::MethodTypeMalformedDescriptor);
                    }
                },
                ConstantPoolEntry::InvokeDynamic { bootstrap_method_attr_index, name_and_type_index } => {
                    let bs_methods = class_file.attributes.get(attrtype::BootstrapMethods);
                    if bs_methods.is_empty() {
                        return Err(ConstantPoolVerificationError::InvokeDynamicNoBootstrapMethodsAttr);
                    }
                    if bs_methods.len() > 1 {
                        return Err(ConstantPoolVerificationError::BootstrapMethodsTooMany);
                    }
                    if let Attributes::BootstrapMethods { bootstrap_methods } = &bs_methods[0] {
                        if *bootstrap_method_attr_index as usize > bootstrap_methods.len() {
                            return Err(ConstantPoolVerificationError::InvokeDynamicInvalidBootstrapMethodsIndex);
                        }
                    }
                    let lexer = Lexer::new();
                    let (name_index, descriptor_index) = match self.get_constant(*name_and_type_index as usize) {
                        ConstantPoolEntry::NameAndType { name_index, descriptor_index } => (name_index, descriptor_index),
                        _ => panic!("Should be impossible, we verified types")                        
                    };

                    let descriptor = self.get_utf8_constant(*descriptor_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;
                    let mut stream = Lexer::stream(lexer.clone(), descriptor.to_string()); 
                    let d = stream.token::<MethodDescriptor>(); 
                    if d.is_err() {
                        return Err(ConstantPoolVerificationError::InvokeDynamicInvalidMethodDescriptor);
                    }

                    let name = self.get_utf8_constant(*name_index as usize).map_err(ConstantPoolVerificationError::ClassFileError)?;

                    let mut stream = Lexer::stream(lexer, name.to_string());
                    if stream.token::<MethodName>().is_err() {
                        return Err(ConstantPoolVerificationError::InvokeDynamicInvalidMethodName);
                    }
                }
                _ => ()
            }
        }
        Ok(())
    }

    /// Verify all constant pool index types within this constant pool.
    pub fn verify_cp_index_types(&self) -> std::result::Result<(), IndexVerificationError> {
        for (index, entry) in self.entries.iter().enumerate() {
            match entry {
                ConstantPoolEntry::Class { name_index } => verify_index!(
                    index,
                    matches!(
                        self.get_constant(*name_index as usize),
                        ConstantPoolEntry::Utf8 { .. } // name index must be UTF-8
                    ),
                    IndexVerificationErrorType::ClassNameIndexNotUTF8
                )?,
                ConstantPoolEntry::Fieldref {
                    class_index,
                    name_and_type_index,
                } => {
                    verify_index!(index, matches!(self.get_constant(*class_index as usize), ConstantPoolEntry::Class { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_ClassIndexNotClass)?;
                    verify_index!(index, matches!(self.get_constant(*name_and_type_index as usize), ConstantPoolEntry::NameAndType { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_NameAndTypeIndexNotNameAndTypeInfo)?;
                }
                ConstantPoolEntry::Methodref {
                    class_index,
                    name_and_type_index,
                } => {
                    verify_index!(index, matches!(self.get_constant(*class_index as usize), ConstantPoolEntry::Class { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_ClassIndexNotClass)?;
                    verify_index!(index, matches!(self.get_constant(*name_and_type_index as usize), ConstantPoolEntry::NameAndType { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_NameAndTypeIndexNotNameAndTypeInfo)?;
                },
                ConstantPoolEntry::InterfaceMethodref {
                    class_index,
                    name_and_type_index,
                } => {
                    verify_index!(index, matches!(self.get_constant(*class_index as usize), ConstantPoolEntry::Class { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_ClassIndexNotClass)?;
                    verify_index!(index, matches!(self.get_constant(*name_and_type_index as usize), ConstantPoolEntry::NameAndType { .. }), IndexVerificationErrorType::InterfaceMethod_Field_Method_ref_NameAndTypeIndexNotNameAndTypeInfo)?;
                },
                ConstantPoolEntry::String { string_index } => verify_index!(index, matches!(self.get_constant(*string_index as usize), ConstantPoolEntry::Utf8 { .. }), IndexVerificationErrorType::StringIndexNotUTF8)?,
                ConstantPoolEntry::NameAndType {
                    name_index,
                    descriptor_index,
                } => {
                    verify_index!(index, matches!(self.get_constant(*name_index as usize), ConstantPoolEntry::Utf8 { .. }), IndexVerificationErrorType::NameAndTypeNameIndexNotUTF8)?;
                    verify_index!(index, matches!(self.get_constant(*descriptor_index as usize), ConstantPoolEntry::Utf8 { .. }), IndexVerificationErrorType::NameAndTypeDescriptorIndexNotUTF8)?;
                },
                ConstantPoolEntry::MethodHandle {
                    reference_kind,
                    reference_index,
                } => {
                    let entry = &self.get_constant(*reference_index as usize);
                    match reference_kind {
                        RefKind::REF_getField | RefKind::REF_getStatic | RefKind::REF_putField | RefKind::REF_putStatic => {
                            verify_index!(index, matches!(entry, ConstantPoolEntry::Fieldref { .. }), IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                        },
                        RefKind::REF_invokeVirtual | RefKind::REF_newInvokeSpecial => {
                            verify_index!(index, matches!(entry, ConstantPoolEntry::Methodref { .. }), IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                            
                            if matches!(reference_kind, RefKind::REF_newInvokeSpecial) {
                                if let ConstantPoolEntry::Methodref { name_and_type_index, .. } = entry {
                                    if let ConstantPoolEntry::NameAndType { name_index, .. } = &self.entries[*name_and_type_index as usize] {
                                        if let ConstantPoolEntry::Utf8 { data } = &self.entries[*name_index as usize] {
                                            if data != "<init>" {
                                                verify_index!(index, false, IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        RefKind::REF_invokeStatic | RefKind::REF_invokeSpecial => {
                            // not handling older than java 8 classes
                            verify_index!(index, matches!(entry, ConstantPoolEntry::Methodref { .. }) || matches!(entry, ConstantPoolEntry::InterfaceMethodref { .. }), IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                        },
                        RefKind::REF_invokeInterface => {
                            verify_index!(index, matches!(entry, ConstantPoolEntry::InterfaceMethodref { .. }), IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                        }
                    }
                    match reference_kind {
                        RefKind::REF_invokeVirtual | RefKind::REF_invokeStatic | RefKind::REF_invokeSpecial | RefKind::REF_invokeInterface => {
                            let info_index = match entry {
                                ConstantPoolEntry::Methodref { name_and_type_index, .. } => *name_and_type_index,
                                ConstantPoolEntry::InterfaceMethodref { name_and_type_index, .. } => *name_and_type_index,
                                _ => { verify_index!(index, false, IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?; unreachable!() }
                            };

                            if let ConstantPoolEntry::NameAndType { name_index, .. } = &self.entries[info_index as usize] {
                                if let ConstantPoolEntry::Utf8 { data } = &self.entries[*name_index as usize] {
                                    verify_index!(index, data != "<init>", IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                                    verify_index!(index, data != "<clinit>", IndexVerificationErrorType::MethodHandleReferenceIndexBadType)?;
                                }
                            }
                        }
                        _ => ()
                    }
                },
                ConstantPoolEntry::MethodType { descriptor_index } => verify_index!(index, matches!(self.get_constant(*descriptor_index as usize), ConstantPoolEntry::Utf8 { .. }), IndexVerificationErrorType::MethodTypeDescriptorIndexNotUTF8)?,
                ConstantPoolEntry::InvokeDynamic {
                    name_and_type_index,
                    ..
                } => verify_index!(index, matches!(self.get_constant(*name_and_type_index as usize), ConstantPoolEntry::NameAndType { .. }), IndexVerificationErrorType::InvokeDynamicNameAndTypeIndexNotNameAndType)?,
                _ => ()
            }
        }
        Ok(())
    }
}

use ahash::AHashMap;
use exo_class_file::item::ids::field::{FieldType, ObjectType, ArrayType, BaseType};

use crate::memory::Trace;

use super::{GcPtr, class::{bootstrap::{JVMRawClass}, FieldNameAndType}};


/// All reference types.
#[derive(Clone, Copy, Debug)]
pub enum JVMRefObjectType {
    /// An instance of an object.
    Class(JVMObjectReference),
    /// An array.
    Array(JVMArrayReference),
    /// Null.
    Null
}

impl JVMRefObjectType {
    /// Mark this reference object as reachable.
    pub unsafe fn mark_reachable(&mut self) {
        match self {
            Self::Array(v) => v.mark_reachable(),
            _ => ()
        }
    }
    
}

impl Trace for JVMRefObjectType {
    unsafe fn trace(&self) {
        match self {
            JVMRefObjectType::Class(v) => v.trace(),
            JVMRefObjectType::Array(v) => v.trace(),
            JVMRefObjectType::Null => (),
        }
    }
}


/// An instance of a class
#[derive(Clone, Copy, Debug)]
pub struct JVMObjectReference {
    pub class: JVMClassInstanceTypes,
}
impl JVMObjectReference {

    pub fn equals(&self, other: JVMObjectReference) -> bool {
        self.class.ptr_eq(other.class)
    }
}
impl Trace for JVMObjectReference {
    unsafe fn trace(&self) {
        self.class.trace();
    }
}

#[derive(Clone, Copy, Debug)]
pub enum JVMArrayType {
    Object(GcPtr<JVMRawClass>),
    Int,
    Char
}
impl Into<FieldType> for JVMArrayType {
    fn into(self) -> FieldType {
        match self {
            JVMArrayType::Object(mut v) => match &unsafe{v.get(0)}.name {
                exo_class_file::item::ids::class::ClassRefName::Class(v) => FieldType::ObjectType(ObjectType { class_name: v.clone() }),
                exo_class_file::item::ids::class::ClassRefName::Array(v) => FieldType::ArrayType(ArrayType(Box::new(v.clone()))),
            },
            JVMArrayType::Char => FieldType::BaseType(BaseType::Char),
            JVMArrayType::Int => FieldType::BaseType(BaseType::Int),
        }
    }
}


impl Trace for JVMArrayType {
    unsafe fn trace(&self) {
        match self {
            JVMArrayType::Object(v) => v.trace(),
            _ => ()
        }
    }
}

/// An instance of a class
#[derive(Clone, Copy, Debug)]
pub struct JVMArrayReference {
    pub array_type: JVMArrayType,
    pub array_ptr: GcPtr<JVMValue>
}
impl JVMArrayReference {
    pub unsafe fn mark_reachable(&mut self) {
        self.array_ptr.mark_reachable();
    }

    pub fn equals(&self, other: JVMArrayReference) -> bool {
        self.array_ptr.ptr_eq(other.array_ptr)
    }
}
impl Trace for JVMArrayReference {
    unsafe fn trace(&self) {
        self.array_type.trace();
        self.array_ptr.trace();
    }
}


#[derive(Debug, Clone, Copy)]
pub enum JVMClassInstanceTypes {
    Java(GcPtr<JavaClassInstance>),
    RawClass(GcPtr<JVMRawClass>)
}

impl Trace for JVMClassInstanceTypes {
    unsafe fn trace(&self) {
        match self {
            JVMClassInstanceTypes::Java(v) => v.trace(),
            JVMClassInstanceTypes::RawClass(v) => v.trace(),
        }
    }
}
impl JVMClassInstanceTypes {
    pub fn ptr_eq(&self, other: JVMClassInstanceTypes) -> bool {
        match (self, other) {
            (JVMClassInstanceTypes::Java(v), JVMClassInstanceTypes::Java(v2)) => v.ptr_eq(v2),
            (JVMClassInstanceTypes::RawClass(v), JVMClassInstanceTypes::RawClass(v2)) => v.ptr_eq(v2),
            _ => false
        }
    }
}

/// All JVM values.
#[derive(Clone, Copy, Debug)]
pub enum JVMValue {
    Reference(JVMRefObjectType),
    Int(i32),
    Char(u32)
}
impl Trace for JVMValue {
    unsafe fn trace(&self) {
        if let Self::Reference(v) = self {
            v.trace();
        }
    }
}
#[derive(Debug)]
pub struct JavaClassInstance {
    pub class: GcPtr<JVMRawClass>,
    pub fields: AHashMap<FieldNameAndType, JVMValue>
}

impl Trace for JavaClassInstance {
    unsafe fn trace(&self) {
        self.class.trace()
    }
}
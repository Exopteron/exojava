use exo_class_file::item::constant_pool::ConstantPoolEntry;

use crate::{vm::{GcPtr, object::JVMValue}, memory::Trace};

use super::{bootstrap::JVMRawClass, FieldNameAndType, MethodNameAndType};

/// Runtime constant pool.
#[derive(Debug)]
pub struct RuntimeConstantPool {
    pub pool: Vec<RuntimeConstant>
}

impl Trace for RuntimeConstantPool {
    unsafe fn trace(&self) {
        for v in self.pool.iter() {
            v.trace();
        }
    }
}
impl RuntimeConstantPool {
    pub fn new() -> Self {
        Self { pool: vec![] }
    }
}




#[derive(Debug)]
pub enum RuntimeConstant {
    Class(ConstantClassInfo),
    Field(ConstantFieldRef),
    Method(ConstantMethodRef),
    String(ConstantStringRef),
    Other(ConstantPoolEntry)
}
impl Trace for RuntimeConstant {
    unsafe fn trace(&self) {
        match self {
            Self::Class(v) => v.trace(),
            Self::Field(v) => v.trace(),
            Self::Method(v) => v.trace(),
            Self::String(v) => v.trace(),
            _ => ()
        }
    }
}


/// Runtime constant class info
#[derive(Debug, Clone, Copy)]
pub struct ConstantClassInfo {
    pub class: GcPtr<JVMRawClass>
}
impl Trace for ConstantClassInfo {
    unsafe fn trace(&self) {
        self.class.trace();
    }
}


/// Runtime constant field ref
#[derive(Debug)]
pub struct ConstantFieldRef {
    pub class: GcPtr<JVMRawClass>,
    pub field: FieldNameAndType
}

impl Trace for ConstantFieldRef {
    unsafe fn trace(&self) {
        self.class.trace();
    }
}

/// Runtime constant method ref
#[derive(Debug)]
pub struct ConstantMethodRef {
    pub class: GcPtr<JVMRawClass>,
    pub method: MethodNameAndType
}

impl Trace for ConstantMethodRef {
    unsafe fn trace(&self) {
        self.class.trace();
    }
}

/// Runtime constant string ref
#[derive(Debug)]
pub struct ConstantStringRef {
    pub value: JVMValue
}

impl Trace for ConstantStringRef {
    unsafe fn trace(&self) {
        self.value.trace();
    }
}
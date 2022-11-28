use std::{fmt::Debug, ops::RangeInclusive};

use exo_class_file::item::{
    fields::FieldAccessFlags,
    ids::{
        field::FieldDescriptor,
        method::{MethodDescriptor, MethodName},
        UnqualifiedName,
    },
    methods::MethodAccessFlags, attribute_info::{ExceptionTableEntry, AttributesCollection}, opcodes::InstructionList,
};

use crate::memory::Trace;

use self::bootstrap::JVMRawClass;

use super::{object::JVMValue, Jvm, GcPtr};

pub mod bootstrap;
pub mod constant_pool;

/// Field name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldNameAndType {
    pub name: UnqualifiedName,
    pub descriptor: FieldDescriptor,
}

/// Method name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodNameAndType {
    pub name: MethodName,
    pub descriptor: MethodDescriptor,
}

/// A method implementation.
#[derive(Debug)]
pub struct MethodImplementation {
    pub desc: MethodNameAndType,
    pub access: MethodAccessFlags,
    pub imp: MethodImplementationType,
}
impl MethodImplementation {
    pub fn new(
        desc: MethodNameAndType,
        access: MethodAccessFlags,
        imp: MethodImplementationType,
    ) -> Self {
        Self { desc, access, imp }
    }
}

impl Trace for MethodImplementation {
    unsafe fn trace(&self) {
        self.imp.trace();
    }
}
#[derive(Debug)]
pub enum JVMError {
    /// An exception was thrown.
    Exception(JVMValue),
}

pub type JvmResult<T> = std::result::Result<T, JVMError>;


/// An entry in the exception table.
#[derive(Debug, Clone, Copy)]
pub struct JavaExceptionTableEntry {
    /// The values of the two items start_pc and end_pc indicate
    /// the ranges in the code array at which the exception
    /// handler is active. The value of start_pc must be a
    /// valid index into the code array of the opcode of
    /// an instruction.
    ///
    /// The value of end_pc either must
    /// be a valid index into the code array of the
    /// opcode of an instruction or must be equal to
    /// code_length, the length of the code array.
    /// The value of start_pc must be less than the
    /// value of end_pc
    /// .
    /// The start_pc is inclusive and end_pc is exclusive; that is,
    /// the exception handler must be active while the program
    /// counter is within the interval [start_pc, end_pc).
    pub pc_range: (u16, u16),
    /// The value of the handler_pc item indicates the start of
    /// the exception handler. The value of the item must be a
    /// valid index into the code array and must be the
    /// index of the opcode of an instruction.
    pub handler_pc: u16,
    /**
    If the value of the catch_type item is nonzero, it must be a valid index into the constant_pool table. The constant_pool entry at that index must be a CONSTANT_Class_info structure (§4.4.1) representing a class of exceptions that this exception handler is designated to catch. The exception handler will be called only if the thrown exception is an instance of the given class or one of its subclasses.

    The verifier should check that the class is Throwable or a subclass of Throwable (§4.9.2).

    If the value of the catch_type item is zero, this exception handler is called for all exceptions.

    This is used to implement finally (§3.13).
    **/
    pub catch_type: GcPtr<JVMRawClass>,
}

impl Trace for JavaExceptionTableEntry {
    unsafe fn trace(&self) {
        self.catch_type.trace();
    }
}


#[derive(Debug)]
pub struct JavaMethodCode {
    /// The value of the max_stack item gives the maximum
    /// depth of the operand stack of this method
    /// at any point during execution of the method.
    pub max_stack: u16,
    /// The value of the max_locals item gives the number of
    /// local variables in the local variable array allocated
    /// upon invocation of this method (§2.6.1), including the
    /// local variables used to pass parameters to the method
    /// on its invocation.
    ///
    /// The greatest local variable index for a value of type
    /// long or double is max_locals - 2. The greatest local
    /// variable index for a value of any other type
    /// is max_locals - 1.
    pub max_locals: u16,
    /**
    The code array gives the actual bytes of Java Virtual Machine code
    that implement the method.

    When the code array is read into memory on a byte-addressable machine,
    if the first byte of the array is aligned on a 4-byte boundary, the
    tableswitch and lookupswitch 32-bit offsets will be 4-byte aligned.
    (Refer to the descriptions of those instructions for more
    information on the consequences of code array alignment.)
    **/
    pub code: InstructionList,
    /// Each entry in the exception_table array describes one
    /// exception handler in the code array. The order of the
    /// handlers in the exception_table array is significant.
    pub exception_table: Vec<JavaExceptionTableEntry>,
    /// Each value of the attributes table must be an attribute_info structure (§4.7).
    /// A Code attribute can have any number of optional attributes associated with it.
    pub attributes: AttributesCollection,
}
impl Trace for JavaMethodCode {
    unsafe fn trace(&self) {
        for h in self.exception_table.iter() {
            h.trace();
        }
    }
}



pub enum MethodImplementationType {
    /// A method implemented natively.
    Native(fn(&Jvm, &[JVMValue]) -> JvmResult<Option<JVMValue>>),
    /// A method implemented in bytecode.
    Java {
        code: JavaMethodCode,
        declared_exceptions: Vec<GcPtr<JVMRawClass>>
    }
}
impl Trace for MethodImplementationType {
    unsafe fn trace(&self) {
        if let MethodImplementationType::Java { declared_exceptions, code } = &self {
            for v in declared_exceptions.iter() {
                v.trace();
            }
            code.trace();
        }
    }
}

impl Debug for MethodImplementationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(arg0) => f.debug_tuple("Native").field(&(arg0 as *const _ as usize)).finish(),
            Self::Java { code, declared_exceptions } => f.debug_struct("Java").field("code", code).field("declared_exceptions", declared_exceptions).finish(),
        }
    }
}
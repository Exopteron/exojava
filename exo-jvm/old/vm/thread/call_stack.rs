use exo_class_file::item::opcodes::InstructionList;

use crate::{vm::{object::JVMValue, GcPtr, class::{bootstrap::JVMRawClass, MethodNameAndType, JavaExceptionTableEntry}}, memory::Trace};


/// JVM Call stack
#[derive(Debug)]
pub struct CallStack {
    pub stack: Vec<StackFrame>
}
impl CallStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push_frame(&mut self, f: StackFrame) {
        self.stack.push(f);
    }

    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.stack.pop()
    }

    pub fn top(&mut self) -> Option<&mut StackFrame> {
        self.stack.last_mut()
    }
}

impl Trace for CallStack {
    unsafe fn trace(&self) {
        for v in self.stack.iter() {
            v.trace();
        }
    }
}


/// Call stack frame.
#[derive(Debug)]
pub struct StackFrame {
    /// Operand stack.
    pub operand_stack: Vec<JVMValue>,

    /// Local variables.
    pub local_variables: Vec<JVMValue>,

    /// Currently executing class.
    pub current_class: GcPtr<JVMRawClass>,

    /// Currently executing method info.
    pub current_method: MethodNameAndType,

    /// Exception handlers.
    pub exception_handlers: Vec<JavaExceptionTableEntry>,

    /// Code. `None` in native methods.
    pub code: Option<InstructionList>,

    /// Program counter.
    pub pc: usize
}

impl Trace for StackFrame {
    unsafe fn trace(&self) {
        for v in self.operand_stack.iter() {
            v.trace();
        }
        for v in self.local_variables.iter() {
            v.trace();
        }
        for v in self.exception_handlers.iter() {
            v.trace();
        }
        self.current_class.trace();
    }
}
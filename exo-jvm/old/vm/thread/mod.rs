mod call_stack;
use exo_class_file::item::opcodes::{ArrayTypeCode, VMOpcode, InstructionList};

use crate::{memory::Trace, vm::object::JVMArrayType};

use self::call_stack::{CallStack, StackFrame};

use super::{
    class::{
        bootstrap::JVMRawClass, constant_pool::RuntimeConstant, JVMError, JvmResult,
        MethodImplementation, MethodImplementationType,
    },
    object::{JVMClassInstanceTypes, JVMObjectReference, JVMRefObjectType, JVMValue},
    GcPtr, Jvm,
};

/// A thread of execution.
#[derive(Debug)]
pub struct JVMThread {
    pub call_stack: CallStack,
}

impl JVMThread {
    pub fn new() -> Self {
        Self {
            call_stack: CallStack::new(),
        }
    }

    /// Run some method to completion.
    pub fn run_to_completion(
        &mut self,
        jvm: &Jvm,
        method: GcPtr<MethodImplementation>,
        class: GcPtr<JVMRawClass>,
        arguments: &[JVMValue],
    ) -> JvmResult<Option<JVMValue>> {
        self.setup_method(jvm, method, class, arguments);
        let class = 4;
        'main: loop {
            let cb: JvmResult<Option<JVMValue>> = (|| {
                loop {
                    println!("AE");
                    if self.call_stack.top().unwrap().code.is_none() {
                        println!("No code");
                        let return_v = self.call_stack.top().unwrap().operand_stack.pop();
                        self.call_stack.pop_frame();

                        if let Some(top) = self.call_stack.top() {
                            if let Some(return_v) = return_v {
                                top.operand_stack.push(return_v);
                            }
                        } else if let Some(return_v) = return_v {
                            return Ok(Some(return_v));
                        } else {
                            return Ok(None);
                        }
                        continue;
                    }

                    let stack_frame = if let Some(frame) = self.call_stack.top() {
                        frame
                    } else {
                        panic!("no top frame")
                    };
                    stack_frame.pc += 1;
                    let class = stack_frame.current_class;
                    println!("OPS: {:?}", stack_frame.code.as_ref().unwrap().opcodes[stack_frame.pc - 1]);
                    match &stack_frame.code.as_ref().unwrap().opcodes[stack_frame.pc - 1] {
                        VMOpcode::r#return() => {
                            println!("A");
                            self.call_stack.pop_frame();
                            println!("G");
                            if self.call_stack.top().is_none() {
                                println!("Returning");
                                return Ok(None);
                            }
                            println!("F");
                        }
                        VMOpcode::iconst_5() => {
                            stack_frame.operand_stack.push(JVMValue::Int(5));
                        }
                        VMOpcode::iconst_4() => {
                            stack_frame.operand_stack.push(JVMValue::Int(4));
                        }
                        VMOpcode::iconst_3() => {
                            stack_frame.operand_stack.push(JVMValue::Int(3));
                        }
                        VMOpcode::iconst_2() => {
                            stack_frame.operand_stack.push(JVMValue::Int(2));
                        }
                        VMOpcode::iconst_1() => {
                            stack_frame.operand_stack.push(JVMValue::Int(1));
                        }
                        VMOpcode::iconst_0() => {
                            stack_frame.operand_stack.push(JVMValue::Int(0));
                        }
                        VMOpcode::iconst_m1() => {
                            stack_frame.operand_stack.push(JVMValue::Int(-1));
                        }
                        VMOpcode::aload_0() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[0]);
                        }
                        VMOpcode::aload_1() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[1]);
                        }
                        VMOpcode::aload_2() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[2]);
                        }
                        VMOpcode::aload_3() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[3]);
                        }
                        VMOpcode::aload(v) => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[*v as usize]);
                        }
                        VMOpcode::getfield(idx) => {
                            if let RuntimeConstant::Field(v) = &unsafe { class.get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                let field = stack_frame.operand_stack.pop().unwrap();
                                stack_frame
                                    .operand_stack
                                    .push(jvm.get_field(field, &v.field)?);
                            }
                        }
                        VMOpcode::goto(branch_offset) => {
                            stack_frame.pc = stack_frame.code.as_ref().unwrap().byte_to_code[&((((stack_frame
                                .code.as_ref().unwrap()
                                .code_to_byte[&(stack_frame.pc - 1)])
                                as isize)
                                + *branch_offset as isize)
                                as usize)];
                        }
                        VMOpcode::ifle(branch_offset) => {
                            if let JVMValue::Int(value) = stack_frame.operand_stack.pop().unwrap() {
                                if value <= 0 {
                                    stack_frame.pc = stack_frame.code.as_ref().unwrap().byte_to_code[&((((stack_frame
                                        .code.as_ref().unwrap()
                                        .code_to_byte[&(stack_frame.pc - 1)])
                                        as isize)
                                        + *branch_offset as isize)
                                        as usize)];
                                }
                            }
                        }
                        VMOpcode::if_icmple(branch_offset) => {
                            if let (JVMValue::Int(b), JVMValue::Int(a)) = (
                                stack_frame.operand_stack.pop().unwrap(),
                                stack_frame.operand_stack.pop().unwrap(),
                            ) {
                                if a <= b {
                                    stack_frame.pc = stack_frame.code.as_ref().unwrap().byte_to_code[&((((stack_frame
                                        .code.as_ref().unwrap()
                                        .code_to_byte[&(stack_frame.pc - 1)])
                                        as isize)
                                        + *branch_offset as isize)
                                        as usize)];
                                }
                            }
                        }
                        VMOpcode::invokestatic(idx) => {
                            println!("STACK :{:?}", stack_frame.operand_stack.len());
                            // TODO check superclasses
                            if let RuntimeConstant::Method(v) = &unsafe { (class).get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                let c = v.class;
                                //println!("LOading: {:?}", v.class);
                                println!("SRTACK :{:?}", stack_frame.operand_stack.len());
                                let mut m = jvm.find_method_supers(&v.method, c)?;
                                println!("M :{:?}", m);
                                let nargs =
                                    unsafe { m.get_ref(0) }.desc.descriptor.parameters.len();
                                println!("STACKE :{:?}", stack_frame.operand_stack.len());
                                let mut args = vec![];
                                for _ in 0..nargs {
                                    args.push(
                                        stack_frame.operand_stack.pop().expect("exception l8r"),
                                    );
                                }
                                //stack_frame.pc += 1;
                                self.setup_method(jvm, m, v.class, &args);
                                continue;
                                //jvm.invoke(m, c, arguments)
                            }
                        }
                        VMOpcode::invokevirtual(idx) => {
                            println!("STACK :{:?}", stack_frame.operand_stack.len());
                            // TODO check superclasses
                            if let RuntimeConstant::Method(v) = &unsafe { (class).get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                //println!("LOading: {:?}", v.class);
                                let objectref =
                                    stack_frame.operand_stack.pop().expect("exception l8r");
                                let objectrefclass = if let JVMValue::Reference(
                                    JVMRefObjectType::Class(JVMObjectReference { class }),
                                ) = objectref
                                {
                                    let c = unsafe { class };
                                    match c {
                                        JVMClassInstanceTypes::Java(v) => unsafe{v.get_ref(0)}.class,
                                        JVMClassInstanceTypes::RawClass(v) => todo!(),
                                    }
                                } else {
                                    panic!("not class");
                                };
                                println!("SRTACK :{:?}", stack_frame.operand_stack.len());
                                let mut m = jvm.find_method_supers(&v.method, objectrefclass)?;
                                println!("M :{:?}", m);
                                let nargs =
                                    unsafe { m.get_ref(0) }.desc.descriptor.parameters.len();
                                println!("STACKE :{:?}", stack_frame.operand_stack.len());

                                let mut args = vec![objectref];
                                for _ in 0..nargs {
                                    args.push(
                                        stack_frame.operand_stack.pop().expect("exception l8r"),
                                    );
                                }
                                //stack_frame.pc += 1;
                                self.setup_method(jvm, m, objectrefclass, &args);
                                continue;
                                //jvm.invoke(m, c, arguments)
                            }
                        }
                        VMOpcode::invokespecial(idx) => {
                            // TODO check superclasses
                            if let RuntimeConstant::Method(v) = &unsafe { (class).get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                let c = v.class;
                                println!(
                                    "method: {:?} class {:?}",
                                    v.method,
                                    unsafe { c.get_ref(0) }.name
                                );
                                let mut m = jvm.find_method(&v.method, c)?;
                                println!("m: {:?}", m);
                                let nargs =
                                    unsafe { m.get_ref(0) }.desc.descriptor.parameters.len();
                                let objectref =
                                    stack_frame.operand_stack.pop().expect("exception l8r");
                                let mut args = vec![objectref];
                                for _ in 0..nargs {
                                    args.push(
                                        stack_frame.operand_stack.pop().expect("exception l8r"),
                                    );
                                }
                                //stack_frame.pc += 1;
                                self.setup_method(jvm, m, v.class, &args);
                                continue;
                                //jvm.invoke(m, c, arguments)
                            }
                        }
                        VMOpcode::new(idx) => {
                            if let RuntimeConstant::Class(v) = &unsafe { class.get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                stack_frame
                                    .operand_stack
                                    .push(jvm.blank_class_instance(v.class)?);
                            }
                        }
                        VMOpcode::iload_1() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[1]);
                        }
                        VMOpcode::iload_0() => {
                            stack_frame
                                .operand_stack
                                .push(stack_frame.local_variables[0]);
                        }
                        VMOpcode::astore(n) | VMOpcode::istore(n) => {
                            let v = stack_frame.operand_stack.pop().unwrap();
                            if stack_frame.local_variables.len() <= *n as usize {
                                stack_frame.local_variables.push(v)
                            } else {
                                stack_frame.local_variables[*n as usize] = v;
                            }
                        }
                        VMOpcode::astore_3() | VMOpcode::istore_3() => {
                            let v = stack_frame.operand_stack.pop().unwrap();
                            if stack_frame.local_variables.len() <= 3 {
                                stack_frame.local_variables.push(v)
                            } else {
                                stack_frame.local_variables[3] = v;
                            }
                        }
                        VMOpcode::astore_2() | VMOpcode::istore_2() => {
                            let v = stack_frame.operand_stack.pop().unwrap();
                            if stack_frame.local_variables.len() <= 2 {
                                stack_frame.local_variables.push(v)
                            } else {
                                stack_frame.local_variables[2] = v;
                            }
                        }
                        VMOpcode::astore_1() | VMOpcode::istore_1() => {
                            let v = stack_frame.operand_stack.pop().unwrap();
                            if stack_frame.local_variables.len() <= 1 {
                                stack_frame.local_variables.push(v)
                            } else {
                                stack_frame.local_variables[1] = v;
                            }
                        }
                        VMOpcode::astore_0() | VMOpcode::istore_0() => {
                            let v = stack_frame.operand_stack.pop().unwrap();
                            if stack_frame.local_variables.len() == 0 {
                                stack_frame.local_variables.push(v)
                            } else {
                                stack_frame.local_variables[0] = v;
                            }
                        }
                        VMOpcode::athrow() => {
                            return Err(JVMError::Exception(
                                stack_frame.operand_stack.pop().unwrap(),
                            ));
                        }
                        VMOpcode::bipush(value) => {
                            stack_frame
                                .operand_stack
                                .push(JVMValue::Int((*value as i8) as i32));
                        }
                        VMOpcode::sipush(value) => {
                            stack_frame
                                .operand_stack
                                .push(JVMValue::Int((*value as i16) as i32));
                        }
                        VMOpcode::iadd() => {
                            if let (JVMValue::Int(b), JVMValue::Int(a)) = (
                                stack_frame.operand_stack.pop().unwrap(),
                                stack_frame.operand_stack.pop().unwrap(),
                            ) {
                                stack_frame.operand_stack.push(JVMValue::Int(a + b));
                            }
                        }
                        VMOpcode::dup() => {
                            stack_frame
                                .operand_stack
                                .push(*stack_frame.operand_stack.last().unwrap());
                        }
                        VMOpcode::putfield(idx) => {
                            if let RuntimeConstant::Field(v) = &unsafe { class.get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                let objectref =
                                    stack_frame.operand_stack.pop().expect("Exception latr");

                                let val = stack_frame.operand_stack.pop().expect("Exception latr");
                                println!("v: {:?}", val);
                                jvm.set_field(objectref, &v.field, val)?;
                            } else {
                                panic!("AA");
                            }
                        }
                        VMOpcode::putstatic(idx) => {
                            if let RuntimeConstant::Field(v) = &unsafe { class.get_ref(0) }
                                .runtime_constant_pool
                                .pool[(*idx as usize) - 1]
                            {
                                jvm.set_static_field(
                                    &v.field,
                                    v.class,
                                    stack_frame.operand_stack.pop().expect("Exception latr"),
                                )?;
                            }
                        }
                        VMOpcode::ireturn() | VMOpcode::areturn() => {
                            println!("rturn");
                            let return_v =
                                stack_frame.operand_stack.pop().expect("exception later");
                            self.call_stack.pop_frame();
                            if let Some(top) = self.call_stack.top() {
                                top.operand_stack.push(return_v);
                            } else {
                                return Ok(Some(return_v));
                            }
                        }
                        VMOpcode::anewarray(idx) => {
                            let v = &unsafe { class.get_ref(0) }.runtime_constant_pool.pool[(*idx as usize) - 1];
                            if let RuntimeConstant::Class(v) = v {
                                let size = stack_frame.operand_stack.pop().expect("exception later");
                                if let JVMValue::Int(size) = size {
                                    println!("size: {:?}", size);
                                    let array = jvm.array_instance(JVMArrayType::Object(v.class), size as usize, None)?;
                                    stack_frame.operand_stack.push(array);
                                }
                            }
                        }
                        VMOpcode::aastore() => {
                            let value = stack_frame.operand_stack.pop().expect("exception later");
                            let index = stack_frame.operand_stack.pop().expect("exception later");
                            let array = stack_frame.operand_stack.pop().expect("exception later");
                            if let JVMValue::Reference(JVMRefObjectType::Array(mut array)) = array {

                                if let JVMArrayType::Object(v) = array.array_type {
                                    if let JVMValue::Reference(JVMRefObjectType::Class(obj)) = value {
                                        if let JVMClassInstanceTypes::Java(objclass) = obj.class {
                                            if jvm.is_subclass(unsafe{objclass.get_ref(0)}.class, v) {
                                               // println!("Obj class {:?}", unsafe{unsafe{objclass.get_ref(0)}.class.get_ref(0)}.name);
                                                if let JVMValue::Int(index) = index {
                                                    println!("In");
                                                    if index < 0 || index > array.array_ptr.len() as i32 {
                                                        todo!("index out of bounds exception")
                                                    }
                                                    //println!("Value {} Index {} Len {}", v, index, array.array_ptr.len());
                                                    *unsafe{array.array_ptr.get(index as usize)} = value;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let JVMValue::Reference(JVMRefObjectType::Null) = array {
                                todo!("Null pointer exception")
                            }
                        }
                        VMOpcode::newarray(ty) => {
                            if let JVMValue::Int(count) =
                                stack_frame.operand_stack.pop().expect("exception later")
                            {
                                let ty = match ty {
                                    ArrayTypeCode::T_BOOLEAN => todo!(),
                                    ArrayTypeCode::T_CHAR => todo!(),
                                    ArrayTypeCode::T_FLOAT => todo!(),
                                    ArrayTypeCode::T_DOUBLE => todo!(),
                                    ArrayTypeCode::T_BYTE => todo!(),
                                    ArrayTypeCode::T_SHORT => todo!(),
                                    ArrayTypeCode::T_INT => JVMArrayType::Int,
                                    ArrayTypeCode::T_LONG => todo!(),
                                };
                                stack_frame
                                    .operand_stack
                                    .push(jvm.array_instance(ty, count as usize, None)?)
                            }
                        }
                        VMOpcode::iaload() => {
                            let index = stack_frame.operand_stack.pop().expect("exception later");
                            let array = stack_frame.operand_stack.pop().expect("exception later");
                            if let JVMValue::Reference(JVMRefObjectType::Array(mut array)) = array {
                                if !matches!(array.array_type, JVMArrayType::Int) {
                                    todo!("Bad type");
                                }
                                if let JVMValue::Int(index) = index {
                                    println!("In");
                                    if index < 0 || index > array.array_ptr.len() as i32 {
                                        todo!("index out of bounds exception")
                                    }

                                    stack_frame
                                        .operand_stack
                                        .push(unsafe { *array.array_ptr.get(index as usize) });
                                }
                            } else if let JVMValue::Reference(JVMRefObjectType::Null) = array {
                                todo!("Null pointer exception")
                            }
                        }
                        VMOpcode::iastore() => {
                            let value = stack_frame.operand_stack.pop().expect("exception later");
                            let index = stack_frame.operand_stack.pop().expect("exception later");
                            let array = stack_frame.operand_stack.pop().expect("exception later");
                            if let JVMValue::Reference(JVMRefObjectType::Array(mut array)) = array {
                                if !matches!(array.array_type, JVMArrayType::Int) {
                                    todo!("Bad type");
                                }
                                if let JVMValue::Int(index) = index {
                                    println!("In");
                                    if index < 0 || index > array.array_ptr.len() as i32 {
                                        todo!("index out of bounds exception")
                                    }
                                    if let JVMValue::Int(v) = value {
                                        println!("Value {} Index {} Len {}", v, index, array.array_ptr.len());
                                        *unsafe{array.array_ptr.get(index as usize)} = value;
                                    }
                                }
                            } else if let JVMValue::Reference(JVMRefObjectType::Null) = array {
                                todo!("Null pointer exception")
                            }
                        },
                        VMOpcode::ldc(n) => {
                            let v = &unsafe { class.get_ref(0) }.runtime_constant_pool
                            .pool[(*n as usize) - 1];
                            match v {
                                //RuntimeConstant::Class(v) => stack_frame.operand_stack.push(JVMValue::Reference(JVMRefObjectType::Class(JVMObjectReference { class: v.class }))),
                                RuntimeConstant::Field(_) => todo!(),
                                RuntimeConstant::Method(_) => todo!(),
                                RuntimeConstant::String(v) => stack_frame.operand_stack.push(v.value),
                                RuntimeConstant::Other(_) => todo!(),
                                _ => ()
                            }
                        }
                        v => unimplemented!("{:?}", v),
                    }
                }
            })();

            match cb {
                Ok(v) => return Ok(v),
                Err(JVMError::Exception(JVMValue::Reference(JVMRefObjectType::Class(
                    JVMObjectReference { class },
                )))) => {
                    if let JVMClassInstanceTypes::Java(c) = unsafe { class } {
                        while let Some(frame) = self.call_stack.top() {
                            for handler in &frame.exception_handlers {
                                let range = (frame.code.as_ref().unwrap().byte_to_code[&(handler.pc_range.0 as usize)]
                                    ..frame.code.as_ref().unwrap().byte_to_code[&(handler.pc_range.1 as usize)]);
                                let ty = handler.catch_type;
                                println!(
                                    "Handl err range: {:?} PC {} Handlerpoint {} Opcodelen {}",
                                    range,
                                    frame.pc,
                                    handler.handler_pc,
                                    frame.code.as_ref().unwrap().opcodes.len()
                                );
                                if jvm.is_subclass(unsafe{c.get_ref(0)}.class, ty) && range.contains(&(frame.pc - 1)) {
                                    println!("Within");
                                    frame.pc =
                                        frame.code.as_ref().unwrap().byte_to_code[&(handler.handler_pc as usize)];
                                    frame.operand_stack.push(JVMValue::Reference(
                                        JVMRefObjectType::Class(JVMObjectReference { class }),
                                    ));
                                    continue 'main;
                                }
                            }
                            self.call_stack.pop_frame();
                        }

                        return Err(JVMError::Exception(JVMValue::Reference(
                            JVMRefObjectType::Class(JVMObjectReference { class }),
                        )));
                    }
                    todo!()
                }
                _ => panic!(),
            }
        }
    }

    pub fn setup_method(
        &mut self,
        jvm: &Jvm,
        mut method: GcPtr<MethodImplementation>,
        class: GcPtr<JVMRawClass>,
        arguments: &[JVMValue],
    ) -> bool {
        let m = unsafe { method.get_ref(0) };
        let (code, _exceptions) = match &m.imp {
            MethodImplementationType::Native(v) => {

                self.call_stack.push_frame(StackFrame {
                    operand_stack: Vec::new(),
                    local_variables: Vec::new(),
                    current_class: class,
                    current_method: m.desc.clone(),
                    exception_handlers: Vec::new(),
                    code: None,
                    pc: 0,
                });

                let value = v(jvm, arguments);
                match value {
                    Ok(v) => {
                        if let Some(v) = v {
                            self.call_stack.top().unwrap().operand_stack.push(v);
                        }
                    }
                    Err(e) => todo!(),
                }
                return false;
            }
            MethodImplementationType::Java {
                code,
                declared_exceptions,
            } => (code, declared_exceptions),
        };
        self.call_stack.push_frame(StackFrame {
            operand_stack: Vec::with_capacity(code.max_stack as usize),
            local_variables: Vec::with_capacity(code.max_locals as usize),
            current_class: class,
            current_method: m.desc.clone(),
            exception_handlers: code.exception_table.clone(),
            code: Some(code.code.clone()),
            pc: 0,
        });

        let top_frame = self.call_stack.top().unwrap();
        for (_idx, arg) in arguments.iter().enumerate() {
            top_frame.local_variables.push(*arg);
        }
        return true;
    }
}

impl Trace for JVMThread {
    unsafe fn trace(&self) {
        self.call_stack.trace();
    }
}

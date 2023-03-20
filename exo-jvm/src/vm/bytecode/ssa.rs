use std::{rc::Rc, collections::HashMap, cell::RefCell};

use exo_class_file::item::{opcodes::{InstructionList, VMOpcode}, ConstantPool, constant_pool::ConstantPoolEntry};
use fnv::{FnvHashSet, FnvHashMap};

use crate::vm::bytecode::is_branching;

use super::MethodBlock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    Int,
    Byte,
    Short,
    Long,
    Double,
    Float,
    Char,
    Object,
    Array
}

#[derive(Clone, Copy, Debug)]
pub enum Constant {
    Int(i32),
    Byte(i8),
    Short(i16),
    Long(i64),
    Double(f64),
    Float(f32),
    Char(u16),
    Null
}

impl Showable for Constant {
    fn show(&self, builder: &SSABuilder) -> String {
        match self {
            Constant::Int(v) => format!("{}i32", v),
            Constant::Byte(v) => format!("{}i8", v),
            Constant::Short(v) =>format!("{}i16", v),
            Constant::Long(v) => format!("{}i64", v),
            Constant::Double(v) => format!("{}f64", v),
            Constant::Float(v) => format!("{}f32", v),
            Constant::Char(v) => format!("{}u16", v),
            Constant::Null => format!("null"),
        }
    }
}

impl Constant {
    pub fn ty(&self) -> ValueType {
        match self {
            Self::Int(_) => ValueType::Int,
            Self::Byte(_) => ValueType::Byte,
            Self::Short(_) => ValueType::Short,
            Self::Long(_) => ValueType::Long,
            Self::Double(_) => ValueType::Double,
            Self::Float(_) => ValueType::Float,
            Self::Char(_) => ValueType::Char,
            Self::Null => ValueType::Object,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Operand {
    Constant(Constant),
    Variable(u64)
}
impl Showable for Operand {
    fn show(&self, builder: &SSABuilder) -> String {
        match self {
            Self::Constant(v) => v.show(builder),
            Self::Variable(idx) => {
                let (name, ty) = &builder.variables[*idx as usize];
                format!("%{}:{:?}", name, ty)
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LValue {
    Variable(u64)
}
impl Showable for LValue {
    fn show(&self, builder: &SSABuilder) -> String {
        match self {
            LValue::Variable(idx) => {
                let (name, ty) = &builder.variables[*idx as usize];
                format!("%{}:{:?}", name, ty)
            },
        }
    }
}

pub struct BasicBlock {
    pub name: String,
    pub instructions: Vec<SSAInstruction>,
    pub og_id: u64,
    pub virtual_stack: Vec<(u64, ValueType)>
}

impl BasicBlock {
    pub fn new(name: String, og_id: u64) -> Self {
        Self {
            name,
            instructions: vec![],
            og_id,
            virtual_stack: Vec::new()
        }
    }

    pub fn stack_push(&mut self, var: u64, ty: ValueType) {
        self.virtual_stack.push((var, ty));
    }
    pub fn stack_pop(&mut self) -> (u64, ValueType) {
        self.virtual_stack.pop().unwrap()
    }
    pub fn emit(&mut self, i: SSAInstruction) {
        let last_branch = if let Some(inst) = self.instructions.last_mut() {
            match inst {
                SSAInstruction::Goto(_) | SSAInstruction::Return(_) | SSAInstruction::CompareLE(_, _, _, _) => true,
                _ => false
            }
        } else { false };
        if !last_branch {
            self.instructions.push(i);
        } else {
            self.instructions.insert(self.instructions.len().saturating_sub(2), i);
        }
    }
}

pub enum PoolConstant {
    String(String),
    Phi(Vec<(u64, u64)>)
}
pub struct SSABuilder {
    variables: Vec<(String, ValueType)>,
    var_map: HashMap<String, u64>,
    pub basic_blocks: Vec<Rc<RefCell<BasicBlock>>>,
    jvm_pool: ConstantPool,
    our_pool: Vec<PoolConstant>,
    block_map: HashMap<u64, Rc<RefCell<BasicBlock>>>
}

impl SSABuilder {

    fn new(pool: ConstantPool) -> Self {
        Self {
            jvm_pool: pool,
            var_map: Default::default(),
            basic_blocks: Default::default(),
            variables: Default::default(),
            our_pool: vec![],
            block_map: Default::default()
        
        }
    }

    fn new_string_constant(&mut self, v: String) -> usize {
        let index = self.our_pool.len();
        self.our_pool.push(PoolConstant::String(v));
        index
    }
    fn new_phi_constant(&mut self, v: Vec<(u64, u64)>) -> usize {
        let index = self.our_pool.len();
        self.our_pool.push(PoolConstant::Phi(v));
        index
    }
    fn new_variable(&mut self, name: String, ty: ValueType) -> u64 {
        let v = *self.var_map.entry(name.clone()).and_modify(|v| {*v += 1;}).or_default();
        let index = self.variables.len() as u64;
        self.variables.push((format!("{}{}", name, v), ty));
        index
    }

    fn process_const(&self, v: &VMOpcode) -> Option<Constant> {
        match v {
            VMOpcode::iconst_0() => Some(Constant::Int(0)),
            VMOpcode::iconst_1() => Some(Constant::Int(1)),
            VMOpcode::iconst_2() => Some(Constant::Int(2)),
            VMOpcode::iconst_3() => Some(Constant::Int(3)),
            VMOpcode::iconst_4() => Some(Constant::Int(4)),
            VMOpcode::iconst_5() => Some(Constant::Int(5)),
            VMOpcode::iconst_m1() => Some(Constant::Int(-1)),
            VMOpcode::aconst_null() => Some(Constant::Null),
            VMOpcode::dconst_0() => Some(Constant::Double(0.0)),
            VMOpcode::dconst_1() => Some(Constant::Double(1.0)),
            VMOpcode::ldc(index) => {
                let constant = self.jvm_pool.get_constant(*index as usize).unwrap();
                match constant {
                    ConstantPoolEntry::Integer { bytes } => Some(Constant::Int(*bytes)),
                    ConstantPoolEntry::Double { bytes } => Some(Constant::Double(f64::from_bits(*bytes))),
                    ConstantPoolEntry::Long { bytes } => Some(Constant::Long(*bytes)),
                    _ => panic!()
                }
            }
            _ => None
        }
    }

    fn process_jvm_block(&mut self, process_queue: &mut Vec<u64>, jump_possibilities: &mut FnvHashMap<u64, FnvHashSet<u64>>, m: &MethodBlock, code: &InstructionList, self_block: Rc<RefCell<BasicBlock>>, b: &super::BasicBlock) {
        self_block.borrow_mut().instructions = vec![];
        self_block.borrow_mut().virtual_stack = vec![];
        let og_id = self_block.borrow().og_id;
        let len = m.block_expected_stack[&og_id].len();
        let mut index = 0;
        println!("BALLS {:?}", m.block_expected_stack);
        for v_ty in &m.block_expected_stack[&og_id] {
            println!("ESPECTd ");
            let mut phi = vec![];
            let mut last_ty = None;
            for possibility in jump_possibilities.get(&og_id).unwrap() {
                let v = self.block_map.get(possibility).unwrap().clone();
                println!("{} {} for {} to {}", v.borrow().virtual_stack.len(), len, possibility, og_id);
                let stack_index = (v.borrow().virtual_stack.len() - len) + index;
                let val = v.borrow().virtual_stack[stack_index];
                if let Some(ty) = v_ty {
                    assert_eq!(val.1, *ty);
                }
                if let Some(ty) = last_ty {
                    assert_eq!(val.1, ty);
                } else {
                    last_ty = Some(val.1);
                }
                phi.push((*possibility, val.0));
            }
            let s = self.new_variable("stack".to_string(), last_ty.unwrap());
            self_block.borrow_mut().stack_push(s, last_ty.unwrap());
            self_block.borrow_mut().emit(SSAInstruction::Phi(LValue::Variable(s), self.new_phi_constant(phi)));
            index += 1;
        }
        println!("SUS");
        if let Some((start, end)) = b.start_end {
            println!("S E {} {} for {}", start, end, og_id);
            
            for v in if start != end { start..end } else { if !is_branching(&code.opcodes[start]) {
                start..end + 1
            } else {
                start..end
            } } {
                println!("S E {} {} for {}", start, end, og_id);
                println!("DO IIIIII T {:?} {} {:?}", self_block.borrow_mut().virtual_stack, og_id, b.jump_target);
                println!("IN");
                if let Some(constant) = self.process_const(&code.opcodes[v]) {
                    let s = self.new_variable("stack".to_string(), constant.ty());
                    self_block.borrow_mut().stack_push(s, constant.ty());
                    self_block.borrow_mut().emit(SSAInstruction::Declare(LValue::Variable(s), Operand::Constant(constant)));
                } else {
                    println!("OP {:?}", code.opcodes[v]);
                    match &code.opcodes[v] {
                        VMOpcode::nop() => (),
                        VMOpcode::dup() => {
                            let v = self_block.borrow_mut().stack_pop();
                            self_block.borrow_mut().stack_push(v.0, v.1);
                            self_block.borrow_mut().stack_push(v.0, v.1);
                        }
                        VMOpcode::pop() => {
                            self_block.borrow_mut().stack_pop();
                        }
                        VMOpcode::iadd() => {
                            let mut self_block = self_block.borrow_mut();
                            let b = self_block.stack_pop();
                            let a = self_block.stack_pop();

                            let s = self.new_variable("stack".to_string(), ValueType::Int);
                            self_block.stack_push(s, ValueType::Int);
                            self_block.emit(SSAInstruction::Add(LValue::Variable(s), Operand::Variable(a.0), Operand::Variable(b.0)));
                        }
                        _ => panic!()
                    }
                }
            }
            println!("JA<P {:?} {} {:?}", self_block.borrow_mut().virtual_stack, og_id, b.jump_target);

            macro_rules! horrible_macro_abuse {
                ($e:expr) => {
                    {
                        println!("BENINGINGIOGNG");
                        let expected_stack_len = m.block_expected_stack[&$e].len();
                        println!("ahahhahhah");
                        let our_stack_len = self_block.borrow().virtual_stack.len() ;
                        if expected_stack_len > our_stack_len {
                            let len = expected_stack_len - our_stack_len;
                            for _ in our_stack_len..expected_stack_len {
                                let mut phi = vec![];
                                let mut last_ty = None;
                                for possibility in jump_possibilities.get(&og_id).unwrap() {
                                    let v = self.block_map.get(possibility).unwrap().clone();
                                    println!("{} {} for {} to {}", v.borrow().virtual_stack.len(), len, possibility, og_id);
                                    let stack_index = (v.borrow().virtual_stack.len() - len) + index;
                                    let val = v.borrow().virtual_stack[stack_index];
                                    if let Some(ty) = last_ty {
                                        assert_eq!(val.1, ty);
                                    } else {
                                        last_ty = Some(val.1);
                                    }
                                    phi.push((*possibility, val.0));
                                }
                                let s = self.new_variable("stack".to_string(), last_ty.unwrap());
                                self_block.borrow_mut().stack_push(s, last_ty.unwrap());
                                self_block.borrow_mut().emit(SSAInstruction::Phi(LValue::Variable(s), self.new_phi_constant(phi)));
                                index += 1;
                            }
                        }
                    }
                };
            }
            match b.jump_target {
                None => (),
                Some(super::JumpTarget::Return) => {
                    println!("RERRURN");
                    let stack_top = self_block.borrow_mut().stack_pop();
                    self_block.borrow_mut().emit(SSAInstruction::Return(Operand::Variable(stack_top.0)))
                },
                Some(super::JumpTarget::Unconditional(v, _passthrough)) => {
                    self_block.borrow_mut().emit(SSAInstruction::Goto(v.get()));
                    if jump_possibilities.entry(v.get()).or_default().insert(self_block.borrow().og_id) {
                        process_queue.push(v.get());
                    }
                    horrible_macro_abuse!(v.get());
                    
                }
                Some(super::JumpTarget::Conditional(success, fail)) => {
                    let jump = b.start_end.unwrap().1;
                    match &code.opcodes[jump] {
                        VMOpcode::if_icmple(_) => {
                            let mut self_block = self_block.borrow_mut();
                            let b = self_block.stack_pop();
                            let a = self_block.stack_pop();
                            self_block.emit(SSAInstruction::CompareLE(Operand::Variable(a.0), Operand::Variable(b.0), success.get(), fail.get()));
                            
                            
                            
                            if jump_possibilities.entry(success.get()).or_default().insert(self_block.og_id) {
                                process_queue.push(success.get());
                            }
                            if jump_possibilities.entry(fail.get()).or_default().insert(self_block.og_id) {
                                process_queue.push(fail.get());
                            }
                        }
                        _ => panic!()
                    }
                    horrible_macro_abuse!(success.get());
                    horrible_macro_abuse!(fail.get());
                }
                _ => panic!()
            }
        }
    }

    pub fn process(code: InstructionList, constant_pool: ConstantPool, m: MethodBlock) -> Self {
        let mut s = Self::new(constant_pool);
        
        let entry = BasicBlock::new("entry".to_string(), m.entry);
        let entry_block = s.add_block(entry);
        s.block_map.insert(m.entry, entry_block.clone());

        for (idx, _block) in m.blocks.iter().enumerate() {
            if idx as u64 == m.entry { continue };
            let block = BasicBlock::new(format!("block{}", idx), idx as u64);
            let block_rc = s.add_block(block);
            s.block_map.insert(idx as u64, block_rc.clone());
        }

        let mut queue = Vec::new();
        let mut processed = FnvHashMap::default();
        s.process_jvm_block(&mut queue, &mut processed, &m, &code, entry_block, &m.blocks[m.entry as usize]);
        while !queue.is_empty() {
            let v = queue.pop().unwrap();

            let j_block = m.blocks[v as usize];
            let our_block = s.block_map.get(&v).unwrap().clone();
            s.process_jvm_block(&mut queue, &mut processed, &m, &code, our_block, &j_block);;
        }
        
        s
    }


    pub fn add_block(&mut self, b: BasicBlock) -> Rc<RefCell<BasicBlock>> {
        let b = Rc::new(RefCell::new(b));
        self.basic_blocks.push(b.clone());
        b
    }
}





#[derive(Debug)]
pub enum SSAInstruction {
    Multiply(LValue, Operand, Operand),
    Add(LValue, Operand, Operand),
    Return(Operand),
    InvokeVirtual(u64),
    Declare(LValue, Operand),
    CompareLE(Operand, Operand, u64, u64),
    Goto(u64),
    Phi(LValue, usize)
}
pub trait Showable {
    fn show(&self, builder: &SSABuilder) -> String;
}

impl Showable for SSAInstruction {
    fn show(&self, builder: &SSABuilder) -> String {
        match self {
            SSAInstruction::Multiply(store, a, b) => format!("mul {}, [ {}, {} ]", store.show(builder), a.show(builder), b.show(builder)),
            SSAInstruction::Add(store, a, b) => format!("add {}, [ {}, {} ]", store.show(builder), a.show(builder), b.show(builder)),
            SSAInstruction::Return(value) => format!("ret {}", value.show(builder)),
            SSAInstruction::InvokeVirtual(_) => todo!(),
            SSAInstruction::Declare(lvalue, var) => format!("{} = {}", lvalue.show(builder), var.show(builder)),
            SSAInstruction::CompareLE(a, b, pass, fail) => format!("cmp_le [ {}, {} ], [ pass = blk \"{}\", fail = blk \"{}\" ]", a.show(builder), b.show(builder), builder.block_map.get(pass).unwrap().borrow().name, builder.block_map.get(fail).unwrap().borrow().name),
            SSAInstruction::Goto(v) => format!("goto blk \"{}\"", builder.block_map.get(v).unwrap().borrow().name),
            SSAInstruction::Phi(lv, indx) => {
                let PoolConstant::Phi(v) = &builder.our_pool[*indx] else { return "invalid".to_string() };

                let mut phi_string = String::new();
                let len = v.len();
                for (index, (block, var)) in v.iter().enumerate() {
                    phi_string.push_str(&format!("[ blk \"{}\", {} ]",  builder.block_map.get(block).unwrap().borrow().name, LValue::Variable(*var).show(builder)));
                    if index != len - 1 {
                        phi_string.push_str(", ");
                    }
                }
                format!("phi {}, [ {} ]", lv.show(builder), phi_string)
            },
        }
    } 
}
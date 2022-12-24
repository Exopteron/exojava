
use std::{collections::HashMap, fmt::Debug};

use super::super::{implementation::ThisCollector, collector::{GarbageCollector, Trace, MemoryManager, Visitor}};



pub struct VarAllocator {
    number: usize,
    returned: Vec<usize>
}

impl Default for VarAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl VarAllocator {
    pub fn new() -> Self {
        Self {
            number: 0,
            returned: vec![]
        }
    }
    pub fn alloc_var(&mut self) -> usize {
        if !self.returned.is_empty() {
            return self.returned.pop().unwrap();
        }
        let v = self.number;
        self.number += 1;
        v
    }
    pub fn free_var(&mut self, v: usize) {
        if v < self.number {
            self.returned.push(v);
        } else {
            panic!("Not part of this allocator");
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VarType {
    Local(usize),
    Global(usize)
}

pub struct VarScoperEntry {
    pub items: HashMap<String, VarType>
}

pub struct VarScoper {
    pub stack: Vec<VarScoperEntry>,
    pub alloc: VarAllocator
}
impl Default for VarScoper {
    fn default() -> Self {
        Self::new()
    }
}

impl VarScoper {
    pub fn new() -> Self {
        Self {
            stack: vec![VarScoperEntry {
                items: HashMap::new()
            }],
            alloc: VarAllocator::new()
        }
    }

    pub fn find_var(&self, name: &str) -> Option<VarType> {
        for stack_entry in self.stack.iter().rev() {
            if let Some(v) = stack_entry.items.get(name) {
                return Some(*v);
            }   
        }
        None
    }


    pub fn declare_general(&mut self, name: String, value: VarType) -> VarType {
        self.stack.last_mut().unwrap().items.insert(name, value);
        value
    }

    pub fn declare_var(&mut self, name: String) -> VarType {
        let value = VarType::Local(self.alloc.alloc_var());
        self.stack.last_mut().unwrap().items.insert(name, value);
        value
    }

    pub fn enter_new_scope(&mut self) {
        self.stack.push(VarScoperEntry { items: HashMap::new() })
    }

    pub fn exit_scope(&mut self) {
        for (_, v) in self.stack.pop().unwrap().items {
            if let VarType::Local(v) = v {
                self.alloc.free_var(v);
            }
        }
    }
}






#[derive(Clone, Copy, Debug)]
pub enum Inst {
    Add,
    Sub,
    Divide,
    Mul,
    StoreVar(usize),
    LoadVar(usize),

    Call(usize),
    StoreGlobal(usize),
    LoadGlobal(usize),
    MkArray(usize),
    Push(f64),
    PushNil,
    GetIndex, // pop(index, array) push(value) 
    SetIndex, // pop(value, index, array) push(prevValue)
    Pop,
    Return
}


pub struct FunctionBlock {
    pub insts: Vec<Inst>,
    pub var_alloc: VarScoper
}


type Ptr<T> = <ThisCollector as MemoryManager>::Ptr<T>;



pub enum Value {
    Number(f64),
    Array(Ptr<[Value]>),
    Nil,
    NativeFn(fn(&mut Compiler) -> Value)
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::Array(_) => write!(f, "Array"),
            Self::Nil => write!(f, "Nil"),
            Self::NativeFn(arg0) => f.debug_tuple("NativeFn").field(&(arg0 as *const _ as usize)).finish(),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Self::Number(arg0) => Self::Number(*arg0),
            Self::Array(arg0) => Self::Array(*arg0),
            Self::Nil => Self::Nil,
            Self::NativeFn(arg0) => Self::NativeFn(*arg0),
        }
    }
}
impl Copy for Value {}


impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(v) => Some(*v),
            _ => None
        }
    }

    pub fn as_nil(&self) -> Option<()> {
        match self {
            Self::Nil => Some(()),
            _ => None
        }
    }

    pub fn as_array(&self) -> Option<Ptr<[Value]>> {
        match self {
            Self::Array(v) => Some(*v),
            _ => None
        }
    }

    pub fn as_native_fn(&self) -> Option<fn(&mut Compiler) -> Value> {
        match self {
            Self::NativeFn(v) => Some(*v),
            _ => None
        }
    }
}


impl Trace<ThisCollector> for [Value] {
    fn trace(&mut self, collector: &GarbageCollector<ThisCollector>, visitor: &mut <ThisCollector as super::super::collector::MemoryManager>::VisitorTy) {
        for v in self.iter_mut() {
            visitor.visit_noref(collector, v);
        }
    }
}

impl Trace<ThisCollector> for Value {
    fn trace(&mut self, collector: &GarbageCollector<ThisCollector>, visitor: &mut <ThisCollector as super::super::collector::MemoryManager>::VisitorTy) {
        match self {
            Value::Array(v) => visitor.visit(collector, v),
            Value::Number(_) | Value::Nil | Value::NativeFn(_) => (),
        }
    }
}

pub struct ExecStackEntry {
    stack: Vec<Value>,
    variables: Vec<Value>
}

impl ExecStackEntry {
    pub fn new(var_max: usize) -> Self {
        Self {
            stack: vec![],
            variables: Vec::from_iter(std::iter::repeat(Value::Nil).take(var_max))
        }
    }

    pub fn pop_value(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    pub fn push_value(&mut self, v: Value) {
        self.stack.push(v);
    }
}


impl Trace<ThisCollector> for ExecStackEntry {
    fn trace(&mut self, collector: &GarbageCollector<ThisCollector>, visitor: &mut <ThisCollector as MemoryManager>::VisitorTy) {
        for entry in &mut self.stack {
            visitor.visit_noref(collector, entry);
        }
        for entry in &mut self.variables {
            visitor.visit_noref(collector, entry);
        }
    }
}

pub struct ExecStack {
    pub stack: Vec<ExecStackEntry>
}

impl Default for ExecStack {
    fn default() -> Self {
        Self::new()
    }
}
impl ExecStack {
    pub fn new() -> Self {
        Self {
            stack: vec![]
        }
    }

    pub fn new_frame(&mut self, var_max: usize) {
        self.stack.push(ExecStackEntry::new(var_max));
    }

    pub fn exit_frame(&mut self) {
        self.stack.pop();
    }

    pub fn pop_value(&mut self) -> Value {
        self.stack.last_mut().unwrap().pop_value()
    }

    pub fn push_value(&mut self, v: Value) {
        self.stack.last_mut().unwrap().push_value(v);
    }

    pub fn load_var(&mut self, v: usize) -> Value {
        self.stack.last_mut().unwrap().variables.get(v).copied().unwrap()
    }

    pub fn store_var(&mut self, idx: usize, v: Value) {
        self.stack.last_mut().unwrap().variables[idx] = v;
    }
}


impl Trace<ThisCollector> for ExecStack {
    fn trace(&mut self, collector: &GarbageCollector<ThisCollector>, visitor: &mut <ThisCollector as MemoryManager>::VisitorTy) {
        for entry in &mut self.stack {
            visitor.visit_noref(collector, entry);
        }
    }
}


pub struct Compiler {
    pub fns: Vec<FunctionBlock>,
    pub current_fn: usize,
    pub globals: Vec<Value>,
    pub exec_stack: ExecStack,
    pub gc: GarbageCollector<ThisCollector>
}

impl Trace<ThisCollector> for Compiler {
    fn trace(&mut self, collector: &GarbageCollector<ThisCollector>, visitor: &mut <ThisCollector as MemoryManager>::VisitorTy) {
        for global in &mut self.globals {
            visitor.visit_noref(collector, global);
        }
        visitor.visit_noref(collector, &mut self.exec_stack);
    }
}

impl Compiler {
    pub fn get_current_fn(&mut self) -> &mut FunctionBlock {
        &mut self.fns[self.current_fn]
    }

    pub fn exec_fn(&mut self, idx: usize) -> Value {
        let f = &mut self.fns[idx];
        self.exec_stack.new_frame(f.var_alloc.alloc.number);
        let insts = f.insts.clone();
        for inst in &insts {
            match inst {
                Inst::Add => {
                    let v2 = self.exec_stack.pop_value();
                    let v1 = self.exec_stack.pop_value();
                    let v = v1.as_number().unwrap() + v2.as_number().unwrap();
                    self.exec_stack.push_value(Value::Number(v));
                },
                Inst::Sub => {
                    let v2 = self.exec_stack.pop_value();
                    let v1 = self.exec_stack.pop_value();
                    let v = v1.as_number().unwrap() - v2.as_number().unwrap();
                    self.exec_stack.push_value(Value::Number(v));
                },
                Inst::Divide => {
                    let v2 = self.exec_stack.pop_value();
                    let v1 = self.exec_stack.pop_value();
                    let v = v1.as_number().unwrap() / v2.as_number().unwrap();
                    self.exec_stack.push_value(Value::Number(v));
                },
                Inst::Mul => {
                    let v2 = self.exec_stack.pop_value();
                    let v1 = self.exec_stack.pop_value();
                    let v = v1.as_number().unwrap() * v2.as_number().unwrap();
                    self.exec_stack.push_value(Value::Number(v));
                },
                Inst::StoreVar(v) => {
                    let val = self.exec_stack.pop_value();
                    self.exec_stack.store_var(*v, val);
                },
                Inst::LoadVar(idx) => {
                    let v = self.exec_stack.load_var(*idx);
                    self.exec_stack.push_value(v);
                },
                Inst::Call(num_args) => {
                    let mut args = Vec::with_capacity(*num_args);
                    for _ in 0..*num_args {
                        args.push(self.exec_stack.pop_value());
                    }
                    args.reverse();
                    let fn_obj = self.exec_stack.pop_value();
                    if let Some(v) = fn_obj.as_native_fn() {
                        self.exec_stack.new_frame(0);
                        let len = args.len();
                        for v in args {
                            self.exec_stack.push_value(v);
                        }
                        self.exec_stack.push_value(Value::Number(len as f64));
                        let v = v(self);
                        self.exec_stack.exit_frame();
                        self.exec_stack.push_value(v);
                    } else {
                        panic!("not fn");
                    }
                },
                Inst::StoreGlobal(idx) => {
                    let value = self.exec_stack.pop_value();
                    self.globals[*idx] = value;
                },
                Inst::LoadGlobal(idx) => {
                    self.exec_stack.push_value(self.globals[*idx]);
                },
                Inst::MkArray(len) => {
                    let mut args = Vec::with_capacity(*len);
                    for _ in 0..*len {
                        args.push(self.exec_stack.pop_value());
                    }
                    args.reverse();
                    let array = ThisCollector::allocate_array(&self.gc, &args).unwrap();
                    self.exec_stack.push_value(Value::Array(array));
                },
                Inst::Push(n) => self.exec_stack.push_value(Value::Number(*n)),
                Inst::Pop => { self.exec_stack.pop_value(); },
                Inst::Return => {
                    let v = self.exec_stack.pop_value();
                    self.exec_stack.exit_frame();
                    return v;
                },
                Inst::GetIndex => {
                    let index = self.exec_stack.pop_value().as_number().unwrap() as usize;
                    let array = self.exec_stack.pop_value().as_array().unwrap();
                    if index > array.get(&self.gc).len() {
                        self.exec_stack.push_value(Value::Nil);
                    } else {
                        self.exec_stack.push_value(array.get(&self.gc)[index]);
                    }
                },
                Inst::SetIndex => {
                    let value = self.exec_stack.pop_value();
                    let index = self.exec_stack.pop_value().as_number().unwrap() as usize;
                    let array = self.exec_stack.pop_value().as_array().unwrap();
                    if index > array.get(&self.gc).len() {
                        panic!("index out of bounds");
                    } else {
                        let prev = array.get(&self.gc)[index];
                        array.get_mut(&self.gc)[index] = value;
                        self.exec_stack.push_value(prev);
                    }
                },
                Inst::PushNil => {
                    self.exec_stack.push_value(Value::Nil);
                },
            }
        }
        self.exec_stack.exit_frame();
        Value::Nil
    }
}




#[cfg(test)]
mod tests {

    use std::num::NonZeroUsize;

    use super::super::super::{testlang::{compile::Compiler, parse::{TokenStream, CharStream, NonTerminal, Block}}, collector::{GarbageCollector, MemoryManager, Visitor}, implementation::ThisCollector};

    use super::{FunctionBlock, ExecStack, Value};

    #[test]
    fn gamer_test() {

        let gc = GarbageCollector::new(ThisCollector::new(NonZeroUsize::new(4 * 1_000_000).unwrap()));
        let mut compiler = Compiler {
            fns: vec![FunctionBlock { insts: vec![], var_alloc: super::VarScoper::new() }],
            current_fn: 0,
            globals: vec![],
            gc,
            exec_stack: ExecStack {
                stack: vec![]
            },
        };
        let mut stream = TokenStream::new(&mut compiler, CharStream::new(r#"
        {
            extern numobjects = 0
            extern collectgarbage = 1
            extern print = 2
            extern asserteq = 3
            asserteq(numobjects(), 0)
            var x = [5,1,2,3]
            asserteq(numobjects(), 1)
            var y = [5, 2, 4, x]
            asserteq(numobjects(), 2)
            x = 69
            asserteq(numobjects(), 2)
            y[3] = nil
            asserteq(numobjects(), 2)
            collectgarbage()
            asserteq(numobjects(), 1)
            y = nil
            asserteq(numobjects(), 1)
            collectgarbage()
            asserteq(numobjects(), 0)

            return 0
        }"#.to_string()));

        for t in &stream.tokens {
            println!("Token: {:?}", t.0);
        }

        Block::visit(&mut compiler, &mut stream).unwrap();
    
        for (idx, fnc) in compiler.fns.iter().enumerate() {
            println!("Fn #{}", idx);
            for (idx, inst) in fnc.insts.iter().enumerate() {
                println!("Inst #{}: {:?}", idx, inst);
            }
        }

        compiler.globals.push(Value::NativeFn(|c| {
            return Value::Number(c.gc.0.borrow().num_objects() as f64);
        }));

        compiler.globals.push(Value::NativeFn(|c| {

            let start = c.gc.0.borrow().num_objects();

            ThisCollector::visit_with(&c.gc.clone(), |v| {
                v.visit_noref(&c.gc.clone(), c);
            });


            let end = c.gc.0.borrow().num_objects();
            Value::Number((start - end) as f64)
        }));

        compiler.globals.push(Value::NativeFn(|c| {

            let num_args = c.exec_stack.pop_value().as_number().unwrap() as usize;

            let collector = c.gc.clone();
            for _ in 0..num_args {
                let v = c.exec_stack.pop_value();
                fn visit_v(gc: GarbageCollector<ThisCollector>, v: Value) {
                    match v {
                        Value::Number(v) => print!("{}", v),
                        Value::Array(v) => {
                            print!("[");
                            for v in v.get(&gc).iter() {
                                (visit_v)(gc.clone(), *v);
                                print!(", ");
                            }
                            print!("]")
                        },
                        Value::Nil => print!("nil"),
                        Value::NativeFn(_) => print!("NativeFn"),
                    }
                    print!("")
                }
                visit_v(collector.clone(), v);
            }
            println!();

            Value::Nil
        }));

        compiler.globals.push(Value::NativeFn(|c| {

            let num_args = c.exec_stack.pop_value().as_number().unwrap() as usize;

            if num_args < 2 {
                return Value::Nil;
            }
            let a = c.exec_stack.pop_value();
            let b = c.exec_stack.pop_value();
            
            let equal = match (a, b) {
                (Value::Number(a), Value::Number(b)) => a == b,
                (Value::Array(a), Value::Array(b)) => {
                    a.ptr_eq(b)
                },
                (Value::Nil, Value::Nil) => true,
                (Value::NativeFn(a), Value::NativeFn(b)) => std::ptr::eq(a as *const (), b as *const ()),
                (_, _) => false
            };

            if !equal {
                panic!("assertion failed");
            }


            
            Value::Nil
        }));
        


        let v = compiler.exec_fn(0);
        println!("Got: {:?}", v);



    }
}
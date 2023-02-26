use std::collections::HashMap;

use exo_class_file::item::{
    attribute_info::Attributes,
    file::ClassFile,
    opcodes::{InstructionList, VMOpcode},
};
use fnv::{FnvHashMap, FnvHashSet};
use nonmax::{NonMaxU64, NonMaxUsize};

use crate::{vm::bytecode::ssa::Showable};

use self::ssa::{SSAInstruction, ValueType, SSABuilder};

mod ssa;


pub fn process(v: ClassFile) {
    for method in v.methods {
        let name = v
            .constant_pool
            .get_utf8_constant(method.name_index as usize)
            .unwrap();
        let descriptor = v
            .constant_pool
            .get_utf8_constant(method.descriptor_index as usize)
            .unwrap();
        if name == "doThing" {
            let data = method.attributes.get("Code").first().unwrap();

            let mut byte_to_code = FnvHashMap::default();
            for i in 0..100 {
                byte_to_code.insert(i, i);
            }
            let code = InstructionList {
                opcodes: vec![
                    VMOpcode::iconst_0(),
                    VMOpcode::iconst_1(),
                    VMOpcode::iadd(),
                    VMOpcode::dup(),
                    VMOpcode::iconst_5(),
                    VMOpcode::if_icmple(2),
                    VMOpcode::goto(-5),
                    VMOpcode::ireturn(),
                ],
                byte_to_code: byte_to_code.clone(),
                code_to_byte: byte_to_code,
            };

            let methodblock = MethodBlock::parse(&code).unwrap();

                        for (name, v) in methodblock.blocks.iter().enumerate() {
                print!("#{}", name);
                if methodblock.entry == name as u64 {
                    print!(" (entry)");
                }
                println!(":");
                if let Some((mut start, end)) = v.start_end {
                    if start > code.opcodes.len() - 1 {
                        start = code.opcodes.len() - 1;
                    }
                    for i in start..=end {
                        let op = &code.opcodes[i];
                        println!(" #{}: {:?}", i, op);
                    }
                }
                println!(" #JMP {:?}", v.jump_target);
            }

            println!("\n\n");
            let b = SSABuilder::process(code, v.constant_pool, methodblock);

            for block in &b.basic_blocks {
                let block = block.borrow();
                println!("BLOCK: {}", block.name);
                for (idx, inst) in block.instructions.iter().enumerate() {
                    println!("    #{}: {}", idx, inst.show(&b));
                }
                println!()
            }

            return;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum JumpTarget {
    Conditional(NonMaxU64, NonMaxU64),
    Unconditional(NonMaxU64, bool),
    Return,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct BasicBlock {
    pub start_end: Option<(usize, usize)>,
    pub jump_target: Option<JumpTarget>,
}

fn is_branching(o: &VMOpcode) -> bool {
    match o {
        VMOpcode::goto(_) | VMOpcode::if_icmple(_) | VMOpcode::ireturn() | VMOpcode::r#return() => {
            true
        }
        _ => false,
    }
}

pub struct MethodBlock {
    pub blocks: Vec<BasicBlock>,

    pub block_expected_stack: FnvHashMap<u64, Vec<Option<ValueType>>>,
    pub entry: u64,
}

struct MethodBlockBuilder {
    current_block: BasicBlock,
    basic_block_list: Vec<(BasicBlock, u64)>,
}
impl MethodBlockBuilder {
    fn new() -> Self {
        Self {
            current_block: Default::default(),
            basic_block_list: Vec::new(),
        }
    }
    fn find_block(&mut self, ends_at: usize, excluding: u64) -> Option<&mut BasicBlock> {
        for (block, idx) in &mut self.basic_block_list {
            if let Some((_, end)) = block.start_end {
                if end == ends_at && *idx != excluding {
                    return Some(block);
                }
            }
        }
        None
    }
    fn push_block(&mut self, block: BasicBlock) -> u64 {
        let len = self.basic_block_list.len() as u64;
        self.basic_block_list.push((block, len));
        len
    }
    fn make_block(
        &mut self,
        code: &InstructionList,
        start_end: Option<(usize, usize)>,
        jump: Option<JumpTarget>,
    ) -> u64 {
        let mut block = BasicBlock {
            start_end,
            jump_target: jump,
        };
        let mut alloc = self.basic_block_list.len() as u64;
        let mut real_index = 0;
        let mut to_add = vec![];

        for (v, idx) in &mut self.basic_block_list {
            let Some((v_start, v_end)) = v.start_end else {
                continue;
            };
            let Some((block_start, block_end)) = block.start_end else {
                continue;
            };
            if block_start == v_start && block_end == v_end && jump.is_none() {
                return *idx;
            }
        }
        'epic: loop {
            println!("A: {:?}", block);
            for (_, (v, name)) in self.basic_block_list.iter().enumerate() {
                println!("#{}:", name);
                if let Some((start, end)) = v.start_end {
                    for i in start..=end {
                        let op = &code.opcodes[i];
                        println!(" #{}: {:?}", i, op);
                    }
                }
                println!(" #JMP {:?}", v.jump_target);
            }
            println!("END\n\n");
            for (v, idx) in &mut self.basic_block_list {
                let Some((v_start, v_end)) = v.start_end else {
                    continue;
                };
                let Some((block_start, block_end)) = block.start_end else {
                    continue;
                };

                // if this block starts within our block
                if v_start >= block_start && v_start <= block_end {
                    println!("HIT {:?} {:?}", v, block);
                    // make our block jump to it
                    block.jump_target = Some(JumpTarget::Unconditional(
                        NonMaxU64::new(*idx).unwrap(),
                        true,
                    ));
                    let old_end = block_end;
                    if let Some((_, end)) = &mut block.start_end {
                        // make our block end right before it starts
                        let v = v_start.overflowing_sub(1);
                        *end = v.0;
                        if v.1 {
                            block.start_end = None;
                        }
                    }
                    if v_end < old_end {
                        let r_s = v_end + 1;
                        let r_e = old_end;
                        if let Some(JumpTarget::Conditional(_succeed, fail)) = &mut v.jump_target {
                            to_add.push(BasicBlock {
                                start_end: Some((r_s, r_e)),
                                jump_target: Some(JumpTarget::Unconditional(*fail, false)),
                            });
                            *fail = NonMaxU64::new(alloc).unwrap();
                        } else if let Some(JumpTarget::Unconditional(_, just_passing_through)) = &mut v.jump_target {
                            if *just_passing_through {
                                to_add.push(BasicBlock {
                                    start_end: Some((r_s, r_e)),
                                    jump_target: v.jump_target,
                                });
                                v.jump_target = Some(JumpTarget::Unconditional(
                                    NonMaxU64::new(alloc).unwrap(),
                                    true,
                                ));
                            }
                        } else {
                            to_add.push(BasicBlock {
                                start_end: Some((r_s, r_e)),
                                jump_target: v.jump_target,
                            });
                            v.jump_target = Some(JumpTarget::Unconditional(
                                NonMaxU64::new(alloc).unwrap(),
                                true,
                            ));
                        }
                        alloc += 1;
                    } else if v_end > old_end {
                        if v_start > block_start {
                            let first_block = (block_start, v_start);
                            let second_block = (v_start + 1, old_end);
                            let third_block = (old_end + 1, v_end);

                            let v_target = match v.jump_target {
                                None => None,
                                Some(JumpTarget::Unconditional(_, v)) if v => {
                                    None
                                },
                                Some(v) => Some(v)
                            };
                            to_add.push(BasicBlock { start_end: Some(third_block), jump_target: v_target });
                            let third_block = alloc;
                            alloc += 1;

                            let b_target = match block.jump_target {
                                None => Some(JumpTarget::Unconditional(NonMaxU64::new(third_block).unwrap(), true)),
                                Some(JumpTarget::Unconditional(_, v)) if v => {
                                    Some(JumpTarget::Unconditional(NonMaxU64::new(third_block).unwrap(), true))
                                },
                                Some(v) => Some(v)
                            };
                            to_add.push(BasicBlock { start_end: Some(second_block), jump_target: b_target });
                            let second_block = alloc;
                            alloc += 1;

                            to_add.push(BasicBlock { start_end: Some(first_block), jump_target: Some(JumpTarget::Unconditional(NonMaxU64::new(second_block).unwrap(), true)) });
                            alloc += 1;
                        } else {
                            panic!("reached");
                        }
                    }
                    continue 'epic;
                } else if v_end <= block_end && v_end >= block_start { // if v ends within our block
                    let r_s = block_start;
                    let r_e = v_end;
                    if let Some((start, _)) = &mut block.start_end {
                        *start = r_e + 1;
                        if *start > code.opcodes.len() - 1 {
                            *start = code.opcodes.len() - 1;
                            if let Some((_, end)) = &mut v.start_end {
                                *end -= 1;
                            }

                        }

                    }

                    let og_j = v.jump_target;

                    if og_j.is_none() || matches!(og_j.unwrap(), JumpTarget::Return) {
                        v.jump_target = Some(JumpTarget::Unconditional(
                            NonMaxU64::new(alloc).unwrap(),
                            true,
                        ));
                        break;
                    } else {
                        v.jump_target = og_j;
                    }
                    continue 'epic;
                }
            }
            to_add.push(block);
            real_index = alloc;
            alloc += 1;
            break;
        }
        for v in to_add {
            self.push_block(v);
        }
        println!("E: {:?}", block);
        for (_, (v, name)) in self.basic_block_list.iter().enumerate() {
            println!("#{}:", name);
            if let Some((start, end)) = v.start_end {
                for i in start..=end {
                    let op = &code.opcodes[i];
                    println!(" #{}: {:?}", i, op);
                }
            }
            println!(" #JMP {:?}", v.jump_target);
        }
        println!("END\n\n");
        real_index
    }
    fn build(self) -> Vec<BasicBlock> {
        self.basic_block_list.into_iter().map(|v| v.0).collect()
    }
}
impl MethodBlock {
    fn find_first_branch(code: &InstructionList, start: usize) -> usize {
        let mut end = None;
        for v in start..code.opcodes.len() {
            println!("{:?} {}", code.opcodes[v], is_branching(&code.opcodes[v]));
            if is_branching(&code.opcodes[v]) {
                end = Some(v);
                break;
            } else {
                continue;
            }
        }
        end.unwrap()
    }

    fn expected_stack_for(code: &InstructionList, s: usize, e: usize) -> Vec<Option<ValueType>> {
        let mut stack = Vec::new();
        let mut known_stack: Vec<Option<ValueType>> = Vec::new();
        for (_, inst) in code.opcodes.iter().enumerate().skip(s).take((e + 1) - s) {
            match inst {
                VMOpcode::pop() => {
                    if known_stack.pop().is_none() {
                        stack.push(None);
                    }
                },
                VMOpcode::dup() => {
                    if known_stack.pop().is_none() {
                        stack.push(None);
                    }
                    known_stack.push(None);
                    known_stack.push(None);
                }
                VMOpcode::iadd() | VMOpcode::imul() | VMOpcode::idiv() | VMOpcode::ireturn() => {
                    if known_stack.pop().is_none() {
                        stack.push(Some(ValueType::Int));
                    }
                }
                VMOpcode::iconst_0() | VMOpcode::iconst_1() | VMOpcode::iconst_2() | VMOpcode::iconst_3() | VMOpcode::iconst_4() | VMOpcode::iconst_5() | VMOpcode::iconst_m1() => {
                    known_stack.push(Some(ValueType::Int));
                }
                _ => ()
            }
        }
        stack
    }

    fn process(
        already_visited: &mut FnvHashMap<usize, (u64, usize)>,
        block_expected_stack: &mut FnvHashMap<u64, Vec<Option<ValueType>>>,
        code: &InstructionList,
        builder: &mut MethodBlockBuilder,
        s: usize,
        e: usize,
        entry: &mut Option<u64>,
    ) -> u64 {
        println!("S {} E {}", s, e);
        for (idx, inst) in code.opcodes.iter().enumerate().skip(s).take((e + 1) - s) {
            println!("V {}", idx);
            println!("IDE2 {} {:?}", idx, inst);
            match inst {
                VMOpcode::ireturn() | VMOpcode::r#return() => {
                    if let Some((v, v_s)) = already_visited.get(&idx) {
                        if *v_s == s {
                            return *v;
                        }
                    }
                    println!("!!!!!!!!!!!!!!!!Makd {} {}", s, idx);
                    let v = builder.make_block(code, Some((s, idx)), Some(JumpTarget::Return));
                    block_expected_stack.insert(v, Self::expected_stack_for(code, s, idx));
                    println!("\n\n\n\nAB\n\n\n\n {}", idx);
                    already_visited.insert(idx, (v, s));
                    println!("V {}", v);
                    let block = builder.find_block(s, v);
                    if let Some(block) = block {
                        if let Some(JumpTarget::Unconditional(_, just_passing_through)) =
                            block.jump_target
                        {
                            if just_passing_through {
                                block.jump_target = Some(JumpTarget::Unconditional(
                                    NonMaxU64::new(v).unwrap(),
                                    true,
                                ));
                            }
                        }
                    } else if entry.is_none() {
                        println!("NUN");
                        *entry = Some(v);
                    }
                    println!("DUn {}", v);
                    return v;
                }
                VMOpcode::goto(idx_offset) => {
                    if let Some((v, v_s)) = already_visited.get(&idx) {
                        if *v_s == s {
                            return *v;
                        }
                    }
                    let goto_idx = ((*code.code_to_byte.get(&idx).unwrap() as isize)
                        + (*idx_offset as isize)) as usize;
                    let goto_idx = *code.byte_to_code.get(&goto_idx).unwrap();
                    
                    let end = Self::find_first_branch(code, goto_idx);
                    let entry_is_none = entry.is_none();
                    let val;
                    println!("HAI {} {}", end, idx);
                    if end == idx {
                        val = (builder.basic_block_list.len()) as u64;
                    } else {
                        val = (builder.basic_block_list.len() + 1) as u64;
                    }
                    already_visited.insert(idx, (val, s));
                    
                    let goto_part = Self::process(already_visited, block_expected_stack, code, builder, goto_idx, end, entry);

                    block_expected_stack.insert(goto_part, Self::expected_stack_for(code, goto_idx, end));
                    println!("IDE {} {} {}", idx, goto_idx, end);
                    let start_end = if idx == 0 { None } else { Some((s, idx)) };
                    let v = builder.make_block(
                        code,
                        start_end,
                        Some(JumpTarget::Unconditional(
                            NonMaxU64::new(goto_part).unwrap(),
                            false,
                        )),
                    );
                    already_visited.insert(idx, (v, s));
                    if let Some((start, end)) = start_end {
                        block_expected_stack.insert(v, Self::expected_stack_for(code, start, end));
                    }
                    let block = builder.find_block(s, v);
                    if let Some(block) = block {
                        if let Some(JumpTarget::Unconditional(_, just_passing_through)) =
                            block.jump_target
                        {
                            if just_passing_through {
                                block.jump_target = Some(JumpTarget::Unconditional(
                                    NonMaxU64::new(v).unwrap(),
                                    true,
                                ));
                            }
                        }
                    } else if entry_is_none {
                        *entry = Some(v);
                    }

                    return v;
                }
                VMOpcode::if_icmple(idx_offset) => {
                    if let Some((v, v_s)) = already_visited.get(&idx) {
                        if *v_s == s {
                            return *v;
                        }
                    }
                    let goto_idx = ((*code.code_to_byte.get(&idx).unwrap() as isize)
                        + (*idx_offset as isize)) as usize;
                    let goto_idx = *code.byte_to_code.get(&goto_idx).unwrap();
                    let goto_end = Self::find_first_branch(code, goto_idx);
                    let entry_is_none = entry.is_none();

                    already_visited.insert(idx, ((builder.basic_block_list.len() +1) as u64, s));
                    let goto_part = Self::process(already_visited, block_expected_stack, code, builder, goto_idx, goto_end, entry);
                    block_expected_stack.insert(goto_part, Self::expected_stack_for(code, goto_idx, goto_end));

                    let fallthrough_end = Self::find_first_branch(code, idx + 1);

                    let fallthrough_part =
                        Self::process(already_visited, block_expected_stack, code, builder, idx + 1, fallthrough_end, entry);

                        block_expected_stack.insert(fallthrough_part, Self::expected_stack_for(code, idx + 1, fallthrough_end));
                    let v = builder.make_block(
                        code,
                        Some((s, idx)),
                        Some(JumpTarget::Conditional(
                            NonMaxU64::new(goto_part).unwrap(),
                            NonMaxU64::new(fallthrough_part).unwrap(),
                        )),
                    );
                    already_visited.insert(idx, (v, s));
                    block_expected_stack.insert(v, Self::expected_stack_for(code, s, idx));
                    let block = builder.find_block(s, v);
                    if let Some(block) = block {
                        if let Some(JumpTarget::Unconditional(_, just_passing_through)) =
                            block.jump_target
                        {
                            if just_passing_through {
                                block.jump_target = Some(JumpTarget::Unconditional(
                                    NonMaxU64::new(v).unwrap(),
                                    true,
                                ));
                            }
                        }
                    } else if entry_is_none {
                        *entry = Some(v);
                    }
                    already_visited.insert(idx, (v, s));
                    return v;
                }
                v => println!("{:?}", v),
            }
        }
        unreachable!()
    }

    pub fn parse(code: &InstructionList) -> Option<Self> {
        // let Attributes::Code { max_stack, max_locals, code, exception_table, attributes } = a else {
        //     return None;
        // };

        let mut builder = MethodBlockBuilder::new();
        let mut start = 0;
        let mut entry = None;
        let mut block_expected_stack = FnvHashMap::default();
        Self::process(&mut FnvHashMap::default(), &mut block_expected_stack, code, &mut builder, start, code.opcodes.len(), &mut entry);

        Some(Self {
            blocks: builder.build(),
            entry: entry.unwrap(),
            block_expected_stack
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use exo_class_file::{
        item::{file::ClassFile, ClassFileItem},
        stream::ClassFileStream,
    };

    use super::process;

    #[test]
    fn epicah() {
        let mut binding = File::open("../local/OptClass.class").unwrap();
        let mut file = ClassFileStream::new(&mut binding);
        let f = ClassFile::read_from_stream(&mut file, None).unwrap();
        process(f);
    }
}

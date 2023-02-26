use std::alloc::Layout;
use std::{cell::RefCell, rc::Rc, num::NonZeroUsize};


pub mod thread;
pub mod bytecode;
pub mod collector;
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

use fnv::FnvHashMap;
use parking_lot::{Mutex, MutexGuard, RwLock, Condvar, lock_api::MutexGuard as LMutexGuard, lock_api::RawMutex};

use self::collector::LinkedListAllocator;
use self::collector::gc::{VMGcState, GcLockState};
use self::collector::object::GcObject;
use self::collector::structures::GcRef;
use self::thread::ThreadState;





pub struct VM {
    gc: Arc<Mutex<VMGcState>>
}
impl VM {
    /// Creates a new JVM.
    /// Returns the JVM as well as a handle to
    /// the current thread's representation.
    pub fn new<'a>() -> (Self, Arc<Mutex<ThreadState>>) {
        let gc = VMGcState::new();


        let this = Self {
            gc: Arc::new(Mutex::new(gc))
        };

        let state = {
            let mut gc = this.gc.lock();
            let state = ThreadState {
                vm: this.clone(),
                gc_condvar: gc.gc_condvar().clone(),
                id: 0,
                collector_id: gc.collector_id(),
                collection_index: gc.collection_index(),
                gamer_stack: vec![]
            };

    
            let state = Arc::new(Mutex::new(state));
            assert_eq!(gc.add_thread(state.clone()), 0);
            state
        };
        (this, state)
    }
}

impl Clone for VM {
    fn clone(&self) -> Self {
        Self {
            gc: self.gc.clone()
        }
    }
}




#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::vm::collector::structures::{StructureBuilder, FieldDef};

    use super::thread::ThreadLocalHandle;
    use super::VM;

    #[test]
    fn epic_balls() {
        println!("Fs");
        let (vm, this_thread) = VM::new();
        println!("A");
        let mut h = ThreadLocalHandle::new(this_thread.lock());
        h.spawn_thread(|mut s| {
            let mut iter_count = 0;
            loop {
                let mut v = s.allocate_object(5i32);
                v.update(|f| {
                    *f += 1;
                });
                println!("2 Done a {:?} there are now {}", v.load(&s, false), s.state().gamer_stack.len());
                if iter_count % 25 == 0 {
                    s.state_mut().gamer_stack.clear();
                    println!("2 Clearing");
                    iter_count = 1;
                } else {
                    println!("2 Push");
                    s.state_mut().gamer_stack.push(v);
                }
                std::thread::sleep(Duration::from_millis(25));
                iter_count += 1;
            }
        });

        let structure = StructureBuilder::new().add_field(FieldDef::new::<i32>("balls".to_string())).build(); 
        let structure = h.allocate_object(structure);
        let mut v = h.construct_structure(structure);
        let off = v.field_offset("balls");

        unsafe {
            v.store(off, false, 42);
        }

        let mut iter_count = 0;
        loop {
            let v = h.allocate_object(5i32);
            println!("Done a {:?} there are now {}", v.load(&h, false), h.state().gamer_stack.len());
            if iter_count % 25 == 0 {
                h.state_mut().gamer_stack.clear();
                println!("Clearing");
                iter_count = 1;
            } else {
                println!("Push");
                h.state_mut().gamer_stack.push(v);
            }
            std::thread::sleep(Duration::from_millis(25));
            iter_count += 1;
        }
    }
}
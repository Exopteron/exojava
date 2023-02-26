
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration, alloc::Layout};

use parking_lot::{Mutex, MutexGuard, RwLock, Condvar, lock_api::MutexGuard as LMutexGuard, lock_api::RawMutex};

use super::{VM, GcLockState, VMGcState, collector::{structures::{GcRef, Structure, StructureDef}, object::{GcObject, Trace, VisitorImpl}}};

pub struct ThreadState {
    pub vm: VM,
    pub gc_condvar: Arc<GcLockState>,
    pub id: usize,
    pub collector_id: u8,
    pub collection_index: u8,
    pub gamer_stack: Vec<GcRef<i32>>
}
unsafe impl Trace for ThreadState {
    const NEEDS_TRACED: bool = true;

    fn trace(
        &mut self,
        gc: &mut VMGcState,
        visitor: &mut VisitorImpl,
    ) {
        for v in &mut self.gamer_stack {
            visitor.visit_noref(gc, v);
        }
    }
}
unsafe impl GcObject for ThreadState {}

impl ThreadState {
    pub fn new(c: VM, id: usize, collector_id: u8, collection_index: u8) -> Arc<Mutex<Self>> {
        let v = c.gc.lock().gc_condvar().clone();
        Arc::new(Mutex::new(Self { gc_condvar: v, vm: c, id, collection_index, collector_id, gamer_stack: vec![] }))
    }
}

pub struct ThreadLocalHandle<'a> {
    state: MutexGuard<'a, ThreadState>,
}

impl<'a> ThreadLocalHandle<'a> {
    pub fn state(&self) -> &ThreadState {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut ThreadState {
        &mut self.state
    }

    pub fn new(state: MutexGuard<'a, ThreadState>) -> Self {
        Self {
            state
        }
    }

    pub fn spawn_thread(&self, f: impl FnOnce(ThreadLocalHandle<'_>) + Send + 'static) {
        let vm = self.state.vm.clone();
        let mut collector = self.collector_lock(&vm.gc);
        collector.spawn_thread(self.state.vm.clone(), f);
    }

    pub fn construct_structure(&mut self, structure: GcRef<StructureDef>) -> Structure {
        let vm = self.state.vm.clone();
        let mut collector = self.collector_lock(&vm.gc);
        collector.construct_structure(&mut self.state, structure)
    }


    pub fn allocate_object<T: GcObject>(&mut self, object: T) -> GcRef<T> {
        let vm = self.state.vm.clone();
        let mut collector = self.collector_lock(&vm.gc);
        collector.allocate_object(&mut self.state, object)
    }



    fn collector_lock<'b>(&self, vm: &'b Mutex<VMGcState>) -> MutexGuard<'b, VMGcState> {
        loop {
            if self.state.gc_condvar.is_waiting.load(Ordering::SeqCst) {
                let v = self.state.gc_condvar.clone();
                println!("locking v");
                let mut lock = v.mutex.lock();
                println!("locked");
                let raw = unsafe {
                    LMutexGuard::mutex(&self.state).raw()
                };
                unsafe {
                    raw.unlock();
                }
                println!("WAit");
                v.condvar.wait(&mut lock);
                println!("Stopped - locking raw");
                raw.lock();
                println!("raw locked");
            }
            {

                let v = vm.try_lock_for(Duration::from_micros(500));
                let Some(v) = v else {
                    continue;
                };
                return v;
            }
        }
    }

    // pub fn allocate_object<T: GcObject>(&mut self, object: T) -> GcRef<T> {
    //     let vm = self.state.vm.clone();
    //     let v = self.collector_lock(&vm.gc);
    // }


    pub fn perform_collection(&mut self) {
        let vm = self.state.vm.clone();
        let mut collector = self.collector_lock(&vm.gc);
        println!("Grabbed lock for collection");
        collector.collection_run(self.state_mut());
    }
}

impl<'a> Drop for ThreadLocalHandle<'a> {
    fn drop(&mut self) {
        let vm = self.state.vm.clone();
        let mut c = self.collector_lock(&vm.gc);
        c.remove_thread(self.state.id);
    }
}
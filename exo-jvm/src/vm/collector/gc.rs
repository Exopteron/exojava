use std::{
    alloc::Layout,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    }, ptr::NonNull,
};

use fnv::FnvHashMap;
use parking_lot::{Condvar, Mutex};

use crate::vm::{
    collector::{GcRootMeta, Mark},
    thread::{ThreadLocalHandle, ThreadState},
    VM,
};

use super::{object::{GcObject, VisitorImpl}, structures::{GcRef, Structure, StructureDef, StructureMetadata, structure_vtable}, LinkedListAllocator, GcRootVTable};

pub struct VMGcState {
    collector: LinkedListAllocator,
    threads: FnvHashMap<usize, Arc<Mutex<ThreadState>>>,
    gc_condvar: Arc<GcLockState>,
    thread_id: usize,
    freed_ids: Vec<usize>,
    collector_id: u8,
    collection_index: u8,
}
impl VMGcState {
    pub fn collector_id(&self) -> u8 {
        self.collector_id
    }
    pub fn collection_index(&self) -> u8 {
        self.collection_index
    }

    pub fn new() -> Self {
        static COLLECTOR_ID: AtomicU8 = AtomicU8::new(0);
        Self {
            collector: LinkedListAllocator::new(NonZeroUsize::new(16384).unwrap()),
            threads: FnvHashMap::default(),
            thread_id: 0,
            freed_ids: vec![],
            gc_condvar: Arc::new(GcLockState {
                condvar: Condvar::new(),
                mutex: Mutex::new(()),
                is_waiting: AtomicBool::new(false),
            }),
            collection_index: 0,
            collector_id: COLLECTOR_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn construct_structure(&mut self, thread: &mut ThreadState, mut structure: GcRef<StructureDef>) -> Structure {
        // TODO: point of contention - this locks a mutex
        let layout = structure.update(|the_structure| {
            the_structure.layout()
        });
        let (layout, offset) =  Layout::new::<StructureMetadata>().extend(layout).unwrap();
        let layout = layout.pad_to_align();

        let mut ptr = self.allocate_memory(thread, layout, structure_vtable());
        unsafe {
           let mut v = ptr.as_mut().data_ptr_mut::<StructureMetadata>().as_mut().unwrap();
            v.def = structure;
            v.offset = offset;
        }
        Structure::new(ptr)

    }

    fn allocate_memory(&mut self, thread: &mut ThreadState, l: Layout, vtable: GcRootVTable) -> NonNull<GcRootMeta> {
        let mut tries = 0;
        loop {
            unsafe {
                let data = self.collector.alloc(l, vtable);
                if let Some(data) = data {
                    return data;
                } else {
                    self.collection_run(thread);
                    if tries > 1 {
                        panic!("could not allocate object");
                    }
                    tries += 1;
                    continue;
                }
            }
        }
    }

    pub fn allocate_object<T: GcObject>(&mut self, thread: &mut ThreadState, object: T) -> GcRef<T> {
        let mut data = self.allocate_memory(thread, Layout::new::<T>(), GcRootVTable::new::<T>());
        unsafe {
            std::ptr::write(data.as_mut().data_ptr_mut(), object);
        }
    
        let ptr = GcRef::new(data, self.collection_index, self.collector_id);
        return ptr;
    }

    fn trace_thread(&mut self, t: &mut ThreadState) {
        let mut visitor = VisitorImpl;
        visitor.visit_noref(self, t);
    }

    pub fn collection_run(&mut self, owning_thread: &mut ThreadState) {
        self.gc_condvar.is_waiting.store(true, Ordering::SeqCst);
        self.trace_thread(owning_thread);

        let id = owning_thread.id;
        for (&k, thread) in &self.threads.clone() {
            if k != id {
                self.trace_thread(&mut *thread.lock());
            }
        }

        println!("Sweeping");

        // sweep
        let mut o = self.collector.object_head;
        while let Some(mut obj) = o {
            unsafe {
                o = obj.as_mut().list.next;
                if obj.as_mut().mark == Mark::White {
                    (obj.as_mut().vtable.finalizer)(obj, self.collection_index, self.collector_id, owning_thread.vm.clone(), self);
                }
            }
        }

        println!("Swept 1");

        // TODO: use the write barrier instead
        let mut freed_objects = vec![];

        self.trace_thread(owning_thread);

        let id = owning_thread.id;
        for (&k, thread) in &self.threads.clone() {
            if k != id {
                self.trace_thread(&mut *thread.lock());
            }
        }
        println!("Swept 2");

        let mut o = self.collector.object_head;
        while let Some(mut obj) = o {
            unsafe {
                o = obj.as_mut().list.next;
                if obj.as_mut().mark == Mark::White {
                    freed_objects.push(obj);
                } else {
                    obj.as_mut().mark = Mark::White;
                }
            }
        }
        println!("Collecting {:?} objects", freed_objects.len());
        for mut obj in freed_objects {
            unsafe {
                (obj.as_mut().vtable.dropper)(obj);
                self.collector.dealloc(obj);
            }
        }




        self.gc_condvar.is_waiting.store(false, Ordering::SeqCst);

        println!("Notifying");
        let mut threads_wait = self.threads.len() - 1; // -1 for this thread
        loop {
            threads_wait -= self.gc_condvar.condvar.notify_all();
            if threads_wait == 0 {
                break;
            }
        }
    }

    //         pub fn perform_collection(&mut self) {
    //     let vm = self.state.vm.clone();
    //     let collector = self.collector_lock(&vm.gc);
    //     collector.gc_condvar.is_waiting.store(true, Ordering::Release);

    //     Self::do_for(&mut self.state);

    //     for i in 0..collector.threads.len() {
    //         if i != self.state.id {
    //             Self::do_for(&mut collector.threads[i].lock());
    //         }
    //     }

    //     collector.gc_condvar.is_waiting.store(false, Ordering::Release);

    //     let mut threads_wait = collector.threads.len() - 1; // -1 for this thread
    //     loop {
    //         threads_wait -= collector.gc_condvar.condvar.notify_all();
    //         if threads_wait == 0 {
    //             break;
    //         }
    //     }
    // }

    pub fn gc_condvar(&self) -> Arc<GcLockState> {
        self.gc_condvar.clone()
    }

    pub fn spawn_thread(&mut self, vm: VM, f: impl FnOnce(ThreadLocalHandle<'_>) + Send + 'static) {
        let id = self.alloc_thread();
        let state = ThreadState {
            id,
            gc_condvar: self.gc_condvar.clone(),
            vm: vm,
            collector_id: self.collector_id,
            collection_index: self.collection_index,
            gamer_stack: vec![]
        };
        let state = Arc::new(Mutex::new(state));
        self.threads.insert(id, state.clone());
        std::thread::spawn(move || {
            let state = state;
            let v = state.lock();
            let v = ThreadLocalHandle::new(v);
            f(v);
        });
    }

    pub(in super::super::super::vm) fn add_thread(&mut self, v: Arc<Mutex<ThreadState>>) -> usize {
        let i = self.alloc_thread();
        self.threads.insert(i, v);
        i
    }

    fn alloc_thread(&mut self) -> usize {
        let i = if let Some(v) = self.freed_ids.pop() {
            v
        } else {
            self.thread_id += 1;
            self.thread_id - 1
        };
        i
    }

    pub fn remove_thread(&mut self, i: usize) {
        self.threads.remove(&i);
        self.freed_ids.push(i);
    }
}
pub struct GcLockState {
    pub condvar: Condvar,
    pub mutex: Mutex<()>,
    pub is_waiting: AtomicBool,
}

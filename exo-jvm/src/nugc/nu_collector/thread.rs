// use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

// use parking_lot::{Mutex, MutexGuard, RwLock, Condvar, lock_api::MutexGuard as LMutexGuard, lock_api::RawMutex};

// pub struct CollectorState {

// }




// pub struct VMGcState {
//     collector: CollectorState,
//     threads: Vec<Arc<Mutex<ThreadState>>>,
//     gc_condvar: Arc<GcLockState>
// }


// pub struct VM {
//     gc: Arc<Mutex<VMGcState>>
// }
// impl VM {
//     pub fn new<'a>() -> (Self, Arc<Mutex<ThreadState>>) {
//         let gc = VMGcState {
//             collector: CollectorState {  },
//             threads: vec![],
//             gc_condvar: Arc::new(GcLockState { condvar: Condvar::new(), mutex: Mutex::new(()), is_waiting: AtomicBool::new(false) })
//         };


//         let this = Self {
//             gc: Arc::new(Mutex::new(gc))
//         };
//         let state = ThreadState {
//             vm: this.clone(),
//             gc_condvar: this.gc.lock().gc_condvar.clone(),
//             id: 0,
//         };

//         let state = Arc::new(Mutex::new(state));
//         this.gc.lock().threads.push(state.clone());
//         (this, state)
//     }
// }

// impl Clone for VM {
//     fn clone(&self) -> Self {
//         Self {
//             gc: self.gc.clone()
//         }
//     }
// }

// struct GcLockState {
//     condvar: Condvar,
//     mutex: Mutex<()>,
//     is_waiting: AtomicBool
// }

// pub struct ThreadState {
//     vm: VM,
//     gc_condvar: Arc<GcLockState>,
//     id: usize
// }

// impl ThreadState {
//     fn new(c: VM, id: usize) -> Arc<Mutex<Self>> {
//         let v = c.gc.lock().gc_condvar.clone();
//         Arc::new(Mutex::new(Self { gc_condvar: v, vm: c, id }))
//     }
// }

// pub struct ThreadLocalHandle<'a> {
//     state: MutexGuard<'a, ThreadState>,
// }

// impl<'a> ThreadLocalHandle<'a> {
//     pub fn new(state: MutexGuard<'a, ThreadState>) -> Self {
//         Self {
//             state
//         }
//     }

//     pub fn spawn_thread(&self, f: impl FnOnce(ThreadLocalHandle<'_>) + Send + 'static) {
//         let vm = self.state.vm.clone();
//         let mut collector = self.collector_lock(&vm.gc);
//         let id = collector.threads.len();
//         let state = ThreadState {
//             id,
//             gc_condvar: collector.gc_condvar.clone(),
//             vm: vm.clone()
//         };
//         let state = Arc::new(Mutex::new(state));
//         collector.threads.push(state.clone());
//         std::thread::spawn(move || {
//             let state = state;
//             let v = state.lock();
//             let v = ThreadLocalHandle::new(v);
//             f(v);
//         });
//     }

//     fn collector_lock<'b>(&self, vm: &'b Mutex<VMGcState>) -> MutexGuard<'b, VMGcState> {
//         loop {
//             if self.state.gc_condvar.is_waiting.load(Ordering::Acquire) {
//                 let v = self.state.gc_condvar.clone();
//                 let mut lock = v.mutex.lock();
//                 let raw = unsafe {
//                     LMutexGuard::mutex(&self.state).raw()
//                 };
//                 unsafe {
//                     raw.unlock();
//                 }
//                 v.condvar.wait(&mut lock);
//                 raw.lock();
//             }
//             {

//                 let v = vm.try_lock_for(Duration::from_micros(500));
//                 let Some(v) = v else {
//                     continue;
//                 };
//                 return v;
//             }
//         }
//     }

//     pub fn allocate_object(&mut self) -> i32 {
//         let vm = self.state.vm.clone();
//         let v = self.collector_lock(&vm.gc);
//         drop(v);
//         return 5;
//     }

//     fn do_for(v: &mut ThreadState) {
//         println!("Doing for thread {:?}", v.id);
//     }   

//     pub fn perform_collection(&mut self) {
//         let vm = self.state.vm.clone();
//         let collector = self.collector_lock(&vm.gc);
//         collector.gc_condvar.is_waiting.store(true, Ordering::Release);



//         Self::do_for(&mut self.state);

//         for i in 0..collector.threads.len() {
//             if i != self.state.id {
//                 Self::do_for(&mut collector.threads[i].lock());
//             }
//         }

//         collector.gc_condvar.is_waiting.store(false, Ordering::Release);

//         let mut threads_wait = collector.threads.len() - 1; // -1 for this thread
//         loop {
//             threads_wait -= collector.gc_condvar.condvar.notify_all();
//             if threads_wait == 0 {
//                 break;
//             }
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::time::Duration;

//     use super::{VM, ThreadLocalHandle};

//     #[test]
//     fn epic_balls() {
//         let (vm, this_thread) = VM::new();
//         let mut h = ThreadLocalHandle::new(this_thread.lock());
//         h.spawn_thread(move |mut v| {
//             loop {
//                 println!("ALlocated {}", v.allocate_object());
//                 std::thread::sleep(Duration::from_secs(1));
//             }
//         });

//         loop {
//             std::thread::sleep(Duration::from_secs(3));
//             h.perform_collection();
//         }
//     }
// }
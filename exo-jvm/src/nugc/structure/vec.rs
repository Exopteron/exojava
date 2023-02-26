// use std::{mem::MaybeUninit, ops::{Index, IndexMut}};

// use crate::{nugc::{implementation::{GcPtr, NonNullGcPtr}, collector::{TheGc, MemoryManager, Visitor}}, vm::JVM, value::JVMResult};



// pub struct GcVec<T: Trace<TheGc> + Finalize> {
//     allocation: NonNullGcPtr<[MaybeUninit<T>]>,
//     vm: JVM,
//     cap: usize,
//     len: usize
// }  

// impl<T: Trace<TheGc> + Finalize> GcVec<T> {

//     pub const DEFAULT_CAPACITY: usize = 16;
    
//     fn alloc_array(vm: &JVM, size: usize) -> JVMResult<NonNullGcPtr<[MaybeUninit<T>]>> {
//         Ok(vm.gc().allocate_native_array(size).map_err(|_| ())?.promote().unwrap())
//     }

//     pub fn new(vm: JVM) -> JVMResult<Self> {
//         let allocation = Self::alloc_array(&vm, Self::DEFAULT_CAPACITY)?;
//         Ok(Self {
//             allocation,
//             cap: Self::DEFAULT_CAPACITY,
//             len: 0,
//             vm
//         })
//     }

//     pub fn len(&self) -> usize {
//         self.len
//     }

//     pub fn capacity(&self) -> usize {
//         self.cap
//     }

//     /// Returns the index of `val`.
//     pub fn push(&mut self, val: T) -> JVMResult<usize> {
//         self.grow_if_needed()?;
//         let index = self.len;
//         self.len += 1;
//         self.allocation.get_mut(&self.vm.gc())[index] = MaybeUninit::new(val);
//         Ok(index)
//     }

//     pub fn pop(&mut self) -> JVMResult<T> {
//         if self.len == 0 {
//             return Err(());
//         }
//         self.len -= 1;
//         unsafe {
//             Ok(self.allocation.get_mut(&self.vm.gc())[self.len].assume_init_read())
//         }
//     }


//     fn grow_if_needed(&mut self) -> JVMResult<()> {
//         if self.len == self.cap {
//             self.cap *= 2;
//             let array = Self::alloc_array(&self.vm, self.cap)?;
//             unsafe {
//                 std::ptr::copy_nonoverlapping(self.allocation.get_mut(&self.vm.gc()).as_ptr(), array.get_mut(&self.vm.gc()).as_mut_ptr(), self.len);
//             }
//         }
//         Ok(())
//     }
// }

// impl<T: Trace<TheGc> + Finalize> Index<usize> for GcVec<T> {
//     type Output = T;

//     fn index(&self, index: usize) -> &Self::Output {
//         if index > self.len() {
//             panic!("index {} out of bounds for GcVec of length {}", index, self.len());
//         }
//         unsafe {
//             &*(&self.allocation.get(&self.vm.gc())[index] as *const _ as *const T)
//         }
//     }
// }

// impl<T: Trace<TheGc> + Finalize> IndexMut<usize> for GcVec<T> {


//     fn index_mut(&mut self, index: usize) -> &mut Self::Output {
//         if index > self.len() {
//             panic!("index {} out of bounds for GcVec of length {}", index, self.len());
//         }
//         unsafe {
//             &mut *(&mut self.allocation.get_mut(&self.vm.gc())[index] as *mut _ as *mut T)
//         }
//     }
// }

// impl<T: Trace<TheGc> + Finalize> Trace<TheGc> for GcVec<T> {
//     fn trace(&mut self, gc: &crate::nugc::collector::GarbageCollector<TheGc>, visitor: &mut <TheGc as MemoryManager>::VisitorTy) {
//         visitor.mark(gc, self.allocation.inner());
//         for i in 0..self.len() {
//             visitor.visit_noref(gc, &mut self[i]);
//         }
//     }
// }

// impl<T: Trace<TheGc> + Finalize> Finalize for GcVec<T> {
//     unsafe fn finalize(this: NonNullGcPtr<Self>, j: JVM) {
        
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::{vm::JVMBuilder, nugc::{collector::{Visitor, TheGc}, implementation::GcPtr}};

//     use super::GcVec;

//     // impl Trace<TheGc> for i32 {
//     //     fn trace(&mut self, gc: &crate::nugc::collector::GarbageCollector<TheGc>, visitor: &mut <TheGc as crate::nugc::collector::MemoryManager>::VisitorTy) {
            
//     //     }
//     // }

//     #[test]
//     fn vec_test() {
//         let mut jvm = JVMBuilder::new().build();
//         let mut vec: GcVec<GcPtr<i32>> = GcVec::new(jvm.new_ref()).unwrap();

//         vec.push(jvm.gc().allocate(420).unwrap()).unwrap();
//         jvm.visit_with(|v| {
//             v.visit_noref(&jvm.gc(), &mut vec);
//         });

//         assert_eq!(*vec.pop().unwrap().get(&jvm.gc()).unwrap(), 420);
//     }
// }
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AllocationError {
    #[error("No memory available for allocation")]
    NoMemory,
}

pub trait MemoryManager: Sized {
    type Ptr<T: ?Sized>: Copy;

    type VisitorTy: Visitor<Self>;

    fn allocate<T>(
        collector: &GarbageCollector<Self>,
        v: T,
    ) -> std::result::Result<Self::Ptr<T>, AllocationError>;


    
    fn allocate_array<T>(
        collector: &GarbageCollector<Self>,
        v: &[T],
    ) -> std::result::Result<Self::Ptr<[T]>, AllocationError>;


    fn visit_with<'b, F: FnOnce(&mut Self::VisitorTy)>(
        collector: &GarbageCollector<Self>,
        f: F,
    );

    fn collection_index(collector: &GarbageCollector<Self>) -> usize;

    fn collect(collector: &GarbageCollector<Self>);
}

pub trait Visitor<Gc: MemoryManager> {
    fn visit<T: ?Sized + Trace<Gc>>(&mut self, collector: &GarbageCollector<Gc>, object: &mut Gc::Ptr<T>);
    fn mark<T: ?Sized>(&mut self, collector: &GarbageCollector<Gc>, object: &mut Gc::Ptr<T>) -> bool;

    fn visit_noref<T: ?Sized + Trace<Gc>>(&mut self, collector: &GarbageCollector<Gc>, object: &mut T);
}

pub trait Trace<Gc: MemoryManager> {
    fn trace(&mut self, gc: &GarbageCollector<Gc>, visitor: &mut Gc::VisitorTy);
}



pub trait Finalize<'col, Gc: MemoryManager> {
    fn finalize(self);
}

struct Epic<Gc: MemoryManager> {
    ptr: Gc::Ptr<i32>,
}

impl<Gc: MemoryManager> Trace<Gc> for Epic<Gc> {
    fn trace(&mut self, gc: &GarbageCollector<Gc>, visitor: &mut Gc::VisitorTy) {
        visitor.mark(gc, &mut self.ptr);
    }
}

pub struct GarbageCollector<M: MemoryManager>(pub Rc<RefCell<M>>);

impl<M: MemoryManager> GarbageCollector<M> {
    pub fn new(collector: M) -> Self {
        Self(Rc::new(RefCell::new(collector)))
    }

    fn allocate<T>(&self, v: T) -> std::result::Result<M::Ptr<T>, AllocationError> {
        M::allocate(self, v)
    }
}
impl<M: MemoryManager> Clone for GarbageCollector<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
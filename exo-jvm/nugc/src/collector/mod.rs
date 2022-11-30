use std::{cell::RefCell, marker::PhantomData};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AllocationError {
    #[error("No memory available for allocation")]
    NoMemory,
}

pub trait MemoryManager<'c>: Sized + 'c {
    type Ptr<T: ?Sized>: Copy;

    type VisitorTy: Visitor<'c, Self>;

    fn allocate<T>(
        collector: &'c GarbageCollector<'c, Self>,
        v: T,
    ) -> std::result::Result<Self::Ptr<T>, AllocationError>;

    fn allocate_array<T>(
        collector: &'c GarbageCollector<'c, Self>,
        v: &[T],
    ) -> std::result::Result<Self::Ptr<[T]>, AllocationError>;


    fn visit_with<'b, F: FnOnce(&mut Self::VisitorTy)>(
        collector: &'b GarbageCollector<'c, Self>,
        f: F,
    );

    fn collect(collector: &'c GarbageCollector<'c, Self>);
}

pub trait Visitor<'a, Gc: MemoryManager<'a>> {
    fn visit<T: ?Sized + Trace<'a, Gc>>(&mut self, object: &mut Gc::Ptr<T>);
    fn mark<T: ?Sized>(&mut self, object: &mut Gc::Ptr<T>) -> bool;

    fn visit_noref<T: ?Sized + Trace<'a, Gc>>(&mut self, object: &mut T);
}

pub trait Trace<'col, Gc: MemoryManager<'col>> {
    fn trace(&mut self, visitor: &mut Gc::VisitorTy);
}



pub trait Finalize<'col, Gc: MemoryManager<'col>> {
    fn finalize(self);
}

struct Epic<'col, Gc: MemoryManager<'col>> {
    ptr: Gc::Ptr<i32>,
}

impl<'col2, Gc: MemoryManager<'col2>> Trace<'col2, Gc> for Epic<'col2, Gc> {
    fn trace(&mut self, visitor: &mut Gc::VisitorTy) {
        visitor.mark(&mut self.ptr);
    }
}

pub struct GarbageCollector<'v, M: MemoryManager<'v>>(pub RefCell<M>, PhantomData<&'v M>);

impl<'v, M: MemoryManager<'v>> GarbageCollector<'v, M> {
    pub fn new(collector: M) -> Self {
        Self(RefCell::new(collector), PhantomData)
    }

    fn allocate<T>(&'v self, v: T) -> std::result::Result<M::Ptr<T>, AllocationError> {
        M::allocate(self, v)
    }
}

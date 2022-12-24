use std::{cell::RefCell, rc::Rc, alloc::LayoutError};

use thiserror::Error;

use crate::structure::StructureDef;

#[derive(Error, Debug)]
pub enum AllocationError {
    #[error("No memory available for allocation")]
    NoMemory,
    #[error("Layout error: {0}")]
    LayoutError(LayoutError)
}

#[repr(transparent)]
pub struct Structure {
    data: [u8],
}

impl Structure {
    /// # Safety
    /// This method only interprets the bytes stored in the field `field` as a `T` 
    /// and does not ensure that it is actually a `T`. 
    pub unsafe fn interpret_field<T>(&mut self, structure_def: &StructureDef, field: &str) -> Option<&mut T> {
        let f = structure_def.field_offset(field)?;

        assert_eq!(std::mem::size_of::<T>(), f.size);

        let ptr = self.data.as_ptr().add(f.offset);
        Some(&mut *(ptr as *mut T))
    }
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

    fn allocate_structure(
        collector: &GarbageCollector<Self>,
        structure: &StructureDef,
    ) -> std::result::Result<Self::Ptr<Structure>, AllocationError>;


    /// Perform a garbage collection run.
    fn visit_with<F: FnOnce(&mut Self::VisitorTy)>(collector: &GarbageCollector<Self>, f: F);

    fn collector_id(collector: &GarbageCollector<Self>) -> u32;
    fn collection_index(collector: &GarbageCollector<Self>) -> usize;
}

pub trait Visitor<Gc: MemoryManager> {
    fn visit<T: ?Sized + Trace<Gc>>(
        &mut self,
        collector: &GarbageCollector<Gc>,
        object: &mut Gc::Ptr<T>,
    );
    fn mark<T: ?Sized>(
        &mut self,
        collector: &GarbageCollector<Gc>,
        object: &mut Gc::Ptr<T>,
    ) -> bool;

    fn visit_noref<T: ?Sized + Trace<Gc>>(
        &mut self,
        collector: &GarbageCollector<Gc>,
        object: &mut T,
    );
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

    pub fn allocate<T>(
        &self,
        v: T,
    ) -> std::result::Result<M::Ptr<T>, AllocationError> {
        M::allocate(self, v)
    }

    pub fn allocate_array<T>(
        &self,
        v: &[T],
    ) -> std::result::Result<M::Ptr<[T]>, AllocationError> {
        M::allocate_array(self, v)
    }

    pub fn allocate_structure(
        &self,
        structure: &StructureDef,
    ) -> std::result::Result<M::Ptr<Structure>, AllocationError> {
        M::allocate_structure(self, structure)
    }

    pub fn visit_with<F: FnOnce(&mut M::VisitorTy)>(&self, f: F) {
        M::visit_with(self, f)
    }

    pub fn collector_id(&self) -> u32 {
        M::collector_id(self)
    }
    pub fn collection_index(&self) -> usize {
        M::collection_index(self)
    }

}
impl<M: MemoryManager> Clone for GarbageCollector<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

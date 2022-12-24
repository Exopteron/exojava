use std::{alloc::LayoutError, cell::RefCell, rc::Rc};

use exo_class_file::item::ids::field::{FieldType, ArrayType};
use thiserror::Error;

pub type TheGc = ThisCollector;

use crate::{
    nugc::implementation::GcPtr,
    structure::{OffsetSize, StructureDef}, value::{types::{JavaTypes, FieldNameAndType, ExactJavaType, GC_PTR_SIZE, PrimitiveType}, JVMResult, JavaType},
};

use super::implementation::{GcMut, ThisCollector};

#[derive(Error, Debug)]
pub enum AllocationError {
    #[error("No memory available for allocation")]
    NoMemory,
    #[error("Layout error: {0}")]
    LayoutError(LayoutError),
    #[error("Null pointer")]
    NullPointer
}
#[repr(C)]
pub struct Structure {
    schema: GcPtr<StructureDef>,
    data: [u8],
}

#[repr(C)]
pub struct ArrayStructure {
    ty: GcPtr<ExactJavaType>,
    length: u32,
    data: [u8]
}

impl ArrayStructure {
    /// FIXME: remove when the JVM struct is real
    pub fn get_array_value(&mut self, gc: &GarbageCollector<TheGc>, index: usize) -> JVMResult<GcPtr<ArrayStructure>> {
        if index > self.length as usize {
            return Err(());
        }
        if !matches!(*self.ty.get(gc).ok_or(())?, ExactJavaType::Array(_)) {
            return Err(());
        }
        unsafe {
            let v = self.data.as_ptr().add(GC_PTR_SIZE * index) as *const GcPtr<ArrayStructure>;
            Ok(*v)
        }

    }

    /// FIXME: remove when the JVM struct is real
    pub fn set_array_value(&mut self, gc: &GarbageCollector<TheGc>, index: usize, structure: GcPtr<ArrayStructure>) -> JVMResult<()> {
        if index > self.length as usize {
            return Err(());
        }
        if !matches!(*self.ty.get(gc).ok_or(())?, ExactJavaType::Array(_)) {
            return Err(());
        }
        unsafe {
            let v = self.data.as_mut_ptr().add(GC_PTR_SIZE * index) as *mut GcPtr<ArrayStructure>;
            v.write(structure);
            Ok(())
        }

    }

    pub fn get_primitive<T: PrimitiveType>(&mut self, gc: &GarbageCollector<TheGc>, index: usize) -> JVMResult<T> {
        if T::get_type() != (*self.ty.get(gc).ok_or(())?).into() {
            return Err(());
        }
        if index > self.length as usize {
            return Err(());
        }
        unsafe {
            let v = self.data.as_ptr().add(T::get_type().size() * index) as *const T;
            Ok(*v)
        }
    }

    pub fn set_primitive<T: PrimitiveType>(&mut self, gc: &GarbageCollector<TheGc>, index: usize, val: T) -> JVMResult<()> {
        if T::get_type() != (*self.ty.get(gc).ok_or(())?).into() {
            return Err(());
        }
        if index > self.length as usize {
            return Err(());
        }
        unsafe {
            let v = self.data.as_mut_ptr().add(T::get_type().size() * index) as *mut T;
            v.write(val);
            Ok(())
        }
    }
}


impl Trace<TheGc> for ArrayStructure {
    fn trace(&mut self, gc: &GarbageCollector<TheGc>, visitor: &mut <TheGc as MemoryManager>::VisitorTy) {
        // FIXME: should probably movve this into exact type's tracer
        visitor.mark(gc, &mut self.ty);
        if let Some(ty) = self.ty.get(gc) {
            if matches!(*ty, ExactJavaType::Array(_)) {
                for i in 0..(self.length as usize) {
                    let array: &mut GcPtr<ArrayStructure> = unsafe { (self.data.as_ptr().add(i * GC_PTR_SIZE) as *mut GcPtr<ArrayStructure>).as_mut().unwrap() };
                    visitor.visit(gc, array);
                }
            } else if matches!(*ty, ExactJavaType::ClassInstance(_)) {
                todo!();
            }
        }
    }
}


#[derive(Debug, Error)]
pub enum StructureError {
    #[error("{0}")]
    AllocationError(AllocationError),

    #[error("no such native field: {0}")]
    NoNativeField(String)
}

impl Structure {
    unsafe fn gc_ptr_at<T: ?Sized>(&mut self, off: OffsetSize) -> &mut GcPtr<T> {
        (self.data.as_ptr().add(off.offset) as *mut GcPtr<T>).as_mut().unwrap()
    }

    
    pub fn write_native_field<T: 'static>(
        &mut self,
        gc: &GarbageCollector<TheGc>,
        field: &str,
        value: T,
    ) -> Result<(), StructureError> {
        let (id, f) = self
            .schema
            .get(gc)
            .expect("Schema should not be null")
            .native_field_offset(field).ok_or_else(|| StructureError::NoNativeField(field.to_string()))?;
        assert_eq!(id, std::any::TypeId::of::<T>(), "Invalid type!");

        let new_value = gc.allocate(value).map_err(StructureError::AllocationError)?;
        unsafe {
            std::ptr::write(self.data.as_ptr().add(f.offset) as *mut _, new_value);
        }
        Ok(())
        

    }

    /// # Safety
    /// This structure must come from the gc `gc`.
    pub unsafe fn native_field<T: 'static>(
        &mut self,
        gc: &GarbageCollector<TheGc>,
        field: &str,
    ) -> Option<GcMut<'_, T>> {
        let (id, f) = self.schema.get(gc)?.native_field_offset(field)?;

        assert_eq!(id, std::any::TypeId::of::<T>(), "Invalid type!");

        let ptr: &mut GcPtr<T> = unsafe { self.gc_ptr_at::<T>(f) };
        ptr.get_mut(gc)
    }
}
impl Trace<TheGc> for Structure {
    fn trace(
        &mut self,
        gc: &GarbageCollector<TheGc>,
        visitor: &mut <TheGc as MemoryManager>::VisitorTy,
    ) {
        visitor.visit(gc, &mut self.schema);
        if let Some(schema) = self.schema.get(gc) {
            for (_, (_, off, trace)) in schema.native_fields() {
                let p = unsafe { self.gc_ptr_at(off) };
                trace(p, gc, visitor);
            }

            for (field_name, (off, ty)) in schema.java_fields() {

                if let ExactJavaType::ClassInstance(_) = ty {
                    let p: &mut GcPtr<Structure> = unsafe { self.gc_ptr_at(off) };
                    visitor.visit(gc, p);
                } else if let ExactJavaType::Array(_) = ty {
                    let p: &mut GcPtr<ArrayStructure> = unsafe { self.gc_ptr_at(off) };
                    visitor.visit(gc, p);
                }
            }
        }
    }
}

pub trait MemoryManager: Sized {
    type Ptr<T: ?Sized>: Copy;
    type OwnedPtr<T: ?Sized>: Clone;

    type VisitorTy: Visitor<Self>;

    fn allocate<T>(
        collector: &GarbageCollector<Self>,
        v: T,
    ) -> std::result::Result<Self::Ptr<T>, AllocationError>;

    fn allocate_array(
        collector: &GarbageCollector<Self>,
        ty: Self::Ptr<ExactJavaType>,
        size: u32,
    ) -> std::result::Result<Self::Ptr<ArrayStructure>, AllocationError>;

    fn allocate_structure(
        collector: &GarbageCollector<Self>,
        structure: Self::Ptr<StructureDef>,
    ) -> std::result::Result<Self::Ptr<Structure>, AllocationError>;

    fn new_global_ref<T: ?Sized>(
        collector: &GarbageCollector<Self>,
        v: Self::Ptr<T>,
    ) -> std::result::Result<Self::OwnedPtr<T>, AllocationError>;

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

    pub fn allocate<T>(&self, v: T) -> std::result::Result<M::Ptr<T>, AllocationError> {
        M::allocate(self, v)
    }

    pub fn allocate_array(&self, ty: M::Ptr<ExactJavaType>, size: u32) -> std::result::Result<M::Ptr<ArrayStructure>, AllocationError> {
        M::allocate_array(self, ty, size)
    }

    pub fn allocate_structure(
        &self,
        structure: M::Ptr<StructureDef>,
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

    pub fn new_global_ref<T: ?Sized>(
        &self,
        v: M::Ptr<T>,
    ) -> std::result::Result<M::OwnedPtr<T>, AllocationError> {
        M::new_global_ref(self, v)
    }
}
impl<M: MemoryManager> Clone for GarbageCollector<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

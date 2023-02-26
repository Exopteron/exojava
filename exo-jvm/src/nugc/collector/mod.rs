use std::{alloc::LayoutError, cell::RefCell, mem::{MaybeUninit, size_of, align_of}, rc::Rc, ptr::Pointee, sync::{atomic::{Ordering, AtomicU64}, Arc}};

use exo_class_file::item::ids::field::{ArrayType, FieldType};
use parking_lot::Mutex;
use thiserror::Error;

pub type TheGc = ThisCollector;

use crate::{
    nugc::implementation::GcPtr,
    structure::{OffsetSize, StructureDef},
    value::{
        types::{ExactJavaType, FieldNameAndType, JavaTypes, PrimitiveType, GC_PTR_SIZE},
        JVMResult, JavaType,
    },
    vm::JVM,
};

use super::implementation::{GcMut, NonNullGcPtr, ThisCollector, OwnedGcPtr, VisitorTy};

#[derive(Error, Debug)]
pub enum AllocationError {
    #[error("No memory available for allocation")]
    NoMemory,
    #[error("Layout error: {0}")]
    LayoutError(LayoutError),
    #[error("Null pointer")]
    NullPointer,
    #[error("Invalid dynamic size {0}")]
    InvalidDynamicSize(usize),
}
#[repr(C)]
pub struct Structure {
    schema: GcPtr<StructureDef>,
    null_set: AtomicU64,
    data: [u8],
}


unsafe impl GcObject for Structure {
    const MIN_SIZE_ALIGN: (usize, usize) = (size_of::<GcPtr<StructureDef>>() + size_of::<u64>(), align_of::<GcPtr<StructureDef>>());

    const NULLABLE: bool = true;

    const DST: bool = true;

    fn valid_dynamic_size(size: usize) -> bool {
        true
    }

    fn trace(
        &mut self,
        gc: &GarbageCollector,
        visitor: &mut VisitorTy,
    ) {
        todo!()
    }

    fn finalize(this: NonNullGcPtr<Self>, j: JVM) {
        
    }
}

#[derive(Debug, Error)]
pub enum StructureError {
    #[error("{0}")]
    AllocationError(AllocationError),

    #[error("no such native field: {0}")]
    NoNativeField(String),
}

impl Structure {
    pub fn create(gc: &GarbageCollector, def: GcPtr<StructureDef>) -> JVMResult<GcPtr<Self>> {
        let s = def.get(gc).unwrap().size();
        let v = gc.allocate_dst::<Self>(s, s).map_err(|_| ())?;
        let mut ptr = v.get_mut(gc).unwrap();
        ptr.schema = def;
        ptr.null_set = AtomicU64::new(0);
        Ok(v)
    }

    pub fn initialize_field<T: GcObject + Sized>(&self, gc: &GarbageCollector, name: &str, value: T) {
        if !self.is_field_nullable(gc, name) && self.is_field_present(gc, name).unwrap() {
            panic!("Already initialized");
        }

        if !self.is_field_nullable(gc, name) {
            self.set_field_present(gc, name);
        }

        
    }

    fn is_field_nullable(&self, gc: &GarbageCollector, name: &str) -> bool {
        let f = self.schema.get(gc).unwrap();
        f.native_field(name).unwrap().nullable_index.is_none()
    }

    fn set_field_present(&self, gc: &GarbageCollector, name: &str) -> Option<()> {
        let f = self.schema.get(gc)?;
        let i = f.native_field(name)?.nullable_index?.get();
        let bit = 1u64 << i as u64;
        self.null_set.fetch_or(bit, Ordering::Relaxed);
        Some(())
    }
    fn is_field_present(&self, gc: &GarbageCollector, name: &str) -> Option<bool> {
        let f = self.schema.get(gc)?;
        let i = f.native_field(name)?.nullable_index?.get();
        let bit = 1u64 << i as u64;
        Some((self.null_set.load(Ordering::Relaxed) & bit) > 0)
    }
}


pub trait Visitor {
    fn visit<T: ?Sized + GcObject>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut GcPtr<T>,
    );
    fn mark<T: ?Sized>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut GcPtr<T>,
    ) -> bool;

    fn visit_noref<T: ?Sized + GcObject>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut T,
    );
}

/// # Safety
/// MIN_SIZE_ALIGN must be the minimum valid size and alignment of any instance of this type.
pub unsafe trait GcObject {
    const MIN_SIZE_ALIGN: (usize, usize);
    const NULLABLE: bool;
    const DST: bool;

    fn valid_dynamic_size(size: usize) -> bool;
    fn trace(
        &mut self,
        gc: &GarbageCollector,
        visitor: &mut VisitorTy,
    );
    fn finalize(this: NonNullGcPtr<Self>, j: JVM);

    fn vtable() -> GcObjectVtable {
        GcObjectVtable {
            tracer: |self_ptr, gc, tracer| {
                let v: &mut GcPtr<Self> = unsafe { std::mem::transmute(self_ptr) };
                tracer.visit(gc, v);
                let mut this = v.get_mut(gc).unwrap();
                this.trace(gc, tracer);
            },
            finalizer: |self_ptr, jvm| {
                let v: NonNullGcPtr<Self> = unsafe { std::mem::transmute(self_ptr) };
                Self::finalize(v, jvm);
            },
            dropper: |self_ptr, meta| {
                
                let v: *mut Self = unsafe { std::ptr::from_raw_parts_mut(self_ptr, std::mem::transmute_copy::<usize, <Self as Pointee>::Metadata>(&meta)) };
                unsafe {
                    std::ptr::drop_in_place(v);
                }
            },
        }
    }
}


type ObjTraceFn = fn(
    &mut GcPtr<()>,
    gc: &GarbageCollector,
    visitor: &mut VisitorTy,
);

type ObjFinalizerFn = fn(crate::nugc::implementation::NonNullGcPtr<()>, JVM);

type ObjDropFn = fn(*mut (), usize);
#[derive(Clone, Copy)]
pub struct GcObjectVtable {
    pub tracer: ObjTraceFn,
    pub finalizer: ObjFinalizerFn,
    pub dropper: ObjDropFn,
}

pub const fn make_finalizer<F: GcObject + ?Sized>() -> ObjFinalizerFn {
    |this, j| unsafe { F::finalize(std::mem::transmute(this), j) }
}

pub struct GarbageCollector(pub Arc<Mutex<TheGc>>);


impl GarbageCollector {
    pub fn new(collector: TheGc) -> Self {
        Self(Arc::new(Mutex::new(collector)))
    }

    pub fn allocate<T: GcObject + Sized>(
        &self,
        v: T,
    ) -> std::result::Result<GcPtr<T>, AllocationError> {
        TheGc::allocate(self, v)
    }

    pub fn allocate_dst<T: GcObject + ?Sized>(
        &self,
        excess_size: usize,
        meta: usize,
    ) -> std::result::Result<GcPtr<T>, AllocationError> {
        TheGc::allocate_dst(self, excess_size, meta)
    }

    // pub fn allocate_native_array<T>(
    //     &self,
    //     len: usize,
    // ) -> std::result::Result<TheGc::Ptr<[MaybeUninit<T>]>, AllocationError> {
    //     TheGc::allocate_native_array(self, len)
    // }

    pub fn visit_with<F: FnMut(&mut VisitorTy)>(&self, jvm: JVM, f: F) {
        TheGc::visit_with(jvm, f)
    }

    pub fn collector_id(&self) -> u8 {
        TheGc::collector_id(self)
    }
    pub fn collection_index(&self) -> u8 {
        TheGc::collection_index(self)
    }

    pub fn new_global_ref<T: ?Sized>(
        &self,
        v: GcPtr<T>,
    ) -> std::result::Result<OwnedGcPtr<T>, AllocationError> {
        TheGc::new_global_ref(self, v)
    }
}
impl Clone for GarbageCollector {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

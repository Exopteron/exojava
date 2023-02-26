use std::{
    alloc::Layout,
    cell::{Cell, RefMut},
    collections::HashSet,
    marker::PhantomData,
    mem::{size_of, MaybeUninit, align_of},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{NonNull, Pointee},
    sync::atomic::{AtomicU32, Ordering, AtomicUsize, AtomicU8},
};

use crate::{
    structure::StructureDef,
    value::{
        types::{ExactJavaType, JavaTypes, GC_PTR_ALIGN, GC_PTR_SIZE},
        Cast, JVMResult, JavaType,
    },
    vm::{JVM, thread::ThreadHandle},
};

use super::collector::{
    make_finalizer, AllocationError, GarbageCollector, GcObject,
    TheGc, Visitor, GcObjectVtable,
};

use self::linked_list::LinkedListAllocator;

mod linked_list;

struct GlobalObject {
    ref_count: Cell<u32>,
    object: *mut GcRoot,
}

pub struct ThisCollector {
    allocator: LinkedListAllocator,
    objects: Vec<Pin<Box<GcRoot>>>,
    global_objects: Vec<Pin<Box<GlobalObject>>>,
    collection_index: u8,
    collector_id: u8,
}

impl ThisCollector {
    pub fn new(size: NonZeroUsize) -> Self {
        static COLLECTOR_ID: AtomicU8 = AtomicU8::new(0);
        Self {
            allocator: LinkedListAllocator::new(size),
            objects: Vec::new(),
            global_objects: Vec::new(),
            collection_index: 0,
            collector_id: COLLECTOR_ID.fetch_add(1, Ordering::SeqCst),
        }
    }

    pub fn num_objects(&self) -> usize {
        self.objects.len()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Mark {
    White,
    Black,
}

type BorrowFlag = isize;
const UNUSED: BorrowFlag = 0;
const fn is_writing(v: BorrowFlag) -> bool {
    v > UNUSED
}
const fn is_reading(v: BorrowFlag) -> bool {
    v < UNUSED
}

/// Root object.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GcRoot {
    ptr: *mut (),
    meta: usize,
    layout: Layout,
    mark: Mark,
    vtable: GcObjectVtable,
}

impl GcRoot {
    pub fn new(
        ptr: *mut (),
        meta: usize,
        layout: Layout,
        mark: Mark,
        vtable: GcObjectVtable,
    ) -> Self {
        Self {
            ptr,
            layout,
            mark,
            meta: meta,
            vtable,
        }
    }
}

// pub struct GcRef<'a, T: ?Sized> {
//     ptr: GcPtr<T>,
//     r: &'a T,
// }

// impl<'a, T: ?Sized> Drop for GcRef<'a, T> {
//     fn drop(&mut self) {
//         if !self.ptr.is_null() {
//             unsafe { self.ptr.get_root_mut() }
//                 .unwrap()
//                 .borrow_flag
//                 .update(|v| v + 1);
//         }
//     }
// }

// impl<'a, T: ?Sized> Deref for GcRef<'a, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.r
//     }
// }

// pub struct GcMut<'a, T: ?Sized> {
//     ptr: GcPtr<T>,
//     r: &'a mut T,
// }

// impl<'a, T: ?Sized> Drop for GcMut<'a, T> {
//     fn drop(&mut self) {
//         if !self.ptr.is_null() {
//             unsafe { self.ptr.get_root_mut() }
//                 .unwrap()
//                 .borrow_flag
//                 .update(|v| v - 1);
//         }
//     }
// }

// impl<'a, T: ?Sized> Deref for GcMut<'a, T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.r
//     }
// }

// impl<'a, T: ?Sized> DerefMut for GcMut<'a, T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.r
//     }
// }

pub struct PtrVisitor;

impl Visitor for PtrVisitor {
    fn visit<T: ?Sized + GcObject>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut GcPtr<T>,
    ) {
        if self.mark(collector, object) {
            if let Some(mut value) = object.get_mut(collector) {
                value.trace(collector, self);
            }
        }
    }

    fn visit_noref<T: ?Sized + GcObject>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut T,
    ) {
        object.trace(collector, self);
    }

    fn mark<T: ?Sized>(
        &mut self,
        collector: &GarbageCollector,
        object: &mut GcPtr<T>,
    ) -> bool {
        if let Some(ptr) = unsafe { object.get_root_mut() } {
            if ptr.mark == Mark::White {
                ptr.mark = Mark::Black;
                object.set_collection_index(object.collection_index() + 1);
                return true; // was not marked reachable
            } else if object.collection_index() == collector.collection_index().saturating_sub(1) {
                object.set_collection_index(object.collection_index() + 1);
            }
        }
        false // was reachable
    }
}

pub struct OwnedGcPtr<T: ?Sized> {
    ptr: GcPtr<T>,
    obj_loc: NonNull<GlobalObject>,
}

impl<T: ?Sized> OwnedGcPtr<T> {
    fn new(ptr: GcPtr<T>, obj_loc: NonNull<GlobalObject>) -> Self {
        Self { ptr, obj_loc }
    }

    pub fn ptr_eq(&self, other: OwnedGcPtr<T>) -> bool {
        self.ptr.ptr_eq(other.ptr)
    }

    // pub fn get(&self, collector: &GarbageCollector) -> Option<GcRef<'_, T>> {
    //     let mut p = self.ptr;
    //     p.set_collection_index(collector.collection_index());
    //     p.get(collector)
    // }

    // pub fn get_mut(
    //     &self,
    //     collector: &GarbageCollector,
    // ) -> Option<GcMut<'_, T>> {
    //     let mut p = self.ptr;
    //     p.set_collection_index(collector.collection_index());
    //     p.get_mut(collector)
    // }
}

impl<T: ?Sized> Drop for OwnedGcPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let c = &self.obj_loc.as_ref().ref_count;
            c.set(c.get() - 1);
        }
    }
}

impl<T: ?Sized> Clone for OwnedGcPtr<T> {
    fn clone(&self) -> OwnedGcPtr<T> {
        unsafe {
            let c = &self.obj_loc.as_ref().ref_count;
            c.set(c.get() + 1);
        }
        Self {
            ptr: self.ptr,
            obj_loc: self.obj_loc,
        }
    }
}

impl<T: ?Sized> Cast<OwnedGcPtr<T>> for GcPtr<T> {
    fn cast(self, j: &JVM) -> JVMResult<OwnedGcPtr<T>> {
        Ok(j.gc().new_global_ref(self).unwrap())
    }
}
impl<T: ?Sized> JavaType for GcPtr<T> {
    fn size(&self) -> usize {
        std::mem::size_of::<GcPtr<T>>()
    }

    fn align(&self) -> NonZeroUsize {
        NonZeroUsize::new(std::mem::align_of::<GcPtr<T>>()).unwrap()
    }
}

#[repr(transparent)]
pub struct NonNullGcPtr<T: ?Sized>(GcPtr<T>);

impl<T: ?Sized> NonNullGcPtr<T> {
    pub fn inner(&mut self) -> &mut GcPtr<T> {
        &mut self.0
    }

    fn new(v: GcPtr<T>) -> Option<Self> {
        if v.is_null() {
            None
        } else {
            Some(Self(v))
        }
    }

    // pub fn get_mut<'a,>(&self, collector: &GarbageCollector) -> GcMut<'a, T> {
    //     self.0.get_mut(collector).unwrap()
    // }

    // pub fn get<'a,>(&self, collector: &GarbageCollector) -> GcRef<'a, T> {
    //     self.0.get(collector).unwrap()
    // }
}

/// Garbage-collected reference to object.

pub struct GcPtr<T: ?Sized> {
    /// Pointer to root.
    /// First 8 bits are the collector ID.
    /// The next 8 bits are the collection cycle number.
    ptr: u64,
    _m: PhantomData<T>,
}

static_assertions::assert_eq_size!(GcPtr<()>, u64);

unsafe impl<T: ?Sized + GcObject> GcObject for GcPtr<T> {
    const MIN_SIZE_ALIGN: (usize, usize) = (size_of::<GcPtr<()>>(), align_of::<GcPtr<()>>());

    const DST: bool = false;
    const NULLABLE: bool = true;
    fn valid_dynamic_size(_size: usize) -> bool {
        false
    }

    default fn trace(
        &mut self,
        gc: &GarbageCollector,
        visitor: &mut VisitorTy,
    ) {
        visitor.mark(gc, self);
    }

    fn finalize(_this: NonNullGcPtr<Self>, _j: JVM) {
        
    }
}

impl<T: ?Sized + Copy> GcPtr<T> {
    pub fn load(&self, handle: &ThreadHandle, volatile: bool) -> Option<T> {
        if self.is_null() {
            return None;
        }
        self.ensure_same_collector(handle);

        match volatile {
            true => {
                unsafe {
                    let v = std::intrinsics::atomic_load_seqcst(std::intrinsics::atomic_load_seqcst::<*mut T>(self.ptr() as *const *mut T));
                    Some(v)
                }
            }
            false => {
                unsafe {
                    let v = std::intrinsics::atomic_load_relaxed(std::intrinsics::atomic_load_relaxed::<*mut T>(self.ptr() as *const *mut T));
                    Some(v)
                }
            }
        }
    }
}



impl<T: ?Sized> GcPtr<T> {

    const COLLECTOR_ID: u64 = 0b1111111100000000000000000000000000000000000000000000000000000000;

    const COLLECTION_INDEX: u64 = 0b0000000011111111000000000000000000000000000000000000000000000000;


    const COLLECTOR_ID_SHIFT: u64 = 56;
    const COLLECTION_INDEX_SHIFT: u64 = 48;

    pub const NULL: GcPtr<T> = Self {
        ptr: 0,
        _m: PhantomData,
    };

    fn new(ptr: NonNull<GcRoot>, collection_index: u8, collector_id: u8) -> Self {
        
        let mut v = ptr.as_ptr() as u64;
        v &= !Self::COLLECTOR_ID;
        v &= !Self::COLLECTION_INDEX;

        v |= (collector_id as u64) << Self::COLLECTOR_ID_SHIFT;
        v |= (collection_index as u64) << Self::COLLECTION_INDEX_SHIFT;
        Self {
            ptr: v,
            _m: PhantomData,
        }
    }

    pub fn promote(self) -> Option<NonNullGcPtr<T>> {
        NonNullGcPtr::new(self)
    }

    pub fn is_null(&self) -> bool {
        self.ptr_eq(Self::NULL)
    }

    fn ptr(&self) -> *mut GcRoot {
        let mut v = self.ptr;
        v &= !Self::COLLECTOR_ID;
        v &= !Self::COLLECTION_INDEX;
        v as *mut GcRoot
    }

    fn collector_id(&self) -> u8 {
        let mut v = self.ptr;
        v &= Self::COLLECTOR_ID;
        (v >> Self::COLLECTOR_ID_SHIFT) as u8
    }

    fn collection_index(&self) -> u8 {
        let mut v = self.ptr;
        v &= Self::COLLECTION_INDEX;
        (v >> Self::COLLECTION_INDEX_SHIFT) as u8
    }

    fn set_collection_index(&mut self, v: u8) {
        self.ptr &= !Self::COLLECTION_INDEX;
        self.ptr |= (v as u64) << Self::COLLECTION_INDEX_SHIFT;
    }


    unsafe fn get_root(&self) -> Option<&GcRoot> {
        self.ptr().as_ref()
    }

    unsafe fn get_root_mut(&mut self) -> Option<&mut GcRoot> {
        self.ptr().as_mut()
    }

    pub fn ptr_eq(&self, other: GcPtr<T>) -> bool {
        std::ptr::eq(self.ptr(), other.ptr())
    }

    fn ensure_same_collector(&self, c: &ThreadHandle) {
        if self.collector_id() != c.collector_id() {
            panic!(
                "mismatched collector id (wrong collector passed in): {} {}",
                self.collector_id(),
                c.collector_id()
            );
        }
        if self.collection_index() < c.collection_index() {
            panic!(
                "mismatched collection index (potential use-after-free): {} {}",
                self.collection_index(),
                c.collection_index()
            );
        }
    }

    // pub fn get_meta<'a,>(&'a self, collector: &ThreadHandle) -> Option<&'a AtomicUsize> {
    //     if self.is_null() {
    //         return None;
    //     }
    //     self.ensure_same_collector(collector);
    //     let root = unsafe { self.get_root() }?;
    //     Some(&root.meta)
    // }



    // pub fn get<'a,>(
    //     &self,
    //     collector: &ThreadHandle,
    // ) -> Option<GcRef<'a, T>> {
    //     if self.is_null() {
    //         return None;
    //     }
    //     self.ensure_same_collector(collector);
    //     let root = unsafe { self.get_root() }?;
    //     if is_writing(root.borrow_flag.get()) {
    //         panic!("mutably borrowed");
    //     }
    //     root.borrow_flag.update(|v| v - 1);
    //     let meta =
    //         unsafe { std::mem::transmute_copy::<usize, <T as Pointee>::Metadata>(&root.meta.load(Ordering::Relaxed)) };
    //     let ptr = std::ptr::from_raw_parts(root.ptr, meta);
    //     Some(unsafe {
    //         GcRef {
    //             ptr: *self,
    //             r: &*(ptr),
    //         }
    //     })
    // }

    // pub fn get_mut<'a,>(
    //     &self,
    //     collector: &ThreadHandle,
    // ) -> Option<GcMut<'a, T>> {
    //     if self.is_null() {
    //         return None;
    //     }
    //     self.ensure_same_collector(collector);
    //     let root = unsafe { self.get_root() }?;
    //     if is_reading(root.borrow_flag.get()) {
    //         panic!("immutably borrowed");
    //     }
    //     root.borrow_flag.update(|v| v + 1);

    //     let meta =
    //         unsafe { std::mem::transmute_copy::<usize, <T as Pointee>::Metadata>(&root.meta.load(Ordering::Relaxed)) };
    //     let ptr = std::ptr::from_raw_parts_mut(root.ptr, meta);
    //     Some(unsafe {
    //         GcMut {
    //             ptr: *self,
    //             r: &mut *(ptr),
    //         }
    //     })
    // }
}

impl<T: ?Sized> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _m: self._m,
        }
    }
}
impl<T: ?Sized> Copy for GcPtr<T> {}

impl ThisCollector {
    fn calc_remove_list(collector: &mut ThisCollector) -> HashSet<usize> {
        collector.global_objects.retain_mut(|v| {
            let present = v.ref_count.get() > 0;
            if present {
                unsafe {
                    v.object.as_mut().unwrap().mark = Mark::Black;
                }
            }
            present
        });

        let mut remove_list = HashSet::new();
        for (idx, root) in collector.objects.iter_mut().enumerate() {
            if root.mark == Mark::White {
                remove_list.insert(idx);
            }
        }

        for root in collector.objects.iter_mut() {
            root.mark = Mark::White;
        }

        remove_list
    }

    // fn collect(collector: &GarbageCollector) {
    //     let mut collector = collector.0.borrow_mut();
    //     collector.collection_index += 1;

    //     collector.global_objects.retain_mut(|v| {
    //         let present = v.ref_count.get() > 0;
    //         if present {
    //             unsafe {
    //                 v.object.as_mut().unwrap().mark = Mark::Black;
    //             }
    //         }
    //         present
    //     });

    //     let mut remove_list = HashSet::new();
    //     for (idx, root) in collector.objects.iter_mut().enumerate() {
    //         if root.mark == Mark::White {
    //             remove_list.insert(idx);
    //         }
    //     }

    //     for v in &remove_list {
    //         {
    //             let object = &collector.objects[*v];
    //             let ptr = object.ptr as *mut u8;
    //             let layout = object.layout;
    //             unsafe {
    //                 collector.allocator.dealloc(ptr, layout);
    //             }
    //         }
    //     }
    //     let mut new_list = Vec::with_capacity(collector.objects.capacity());
    //     for (idx, v) in std::mem::take(&mut collector.objects)
    //         .into_iter()
    //         .enumerate()
    //     {
    //         if !remove_list.contains(&idx) {
    //             new_list.push(v);
    //         }
    //     }

    //     collector.objects = new_list;

    //     for root in collector.objects.iter_mut() {
    //         root.mark = Mark::White;
    //     }
    // }
}


pub type Ptr<T: ?Sized> = GcPtr<T>;
pub type OwnedPtr<T: ?Sized> = OwnedGcPtr<T>;

pub type VisitorTy = PtrVisitor;
impl ThisCollector {


    // /// UNMANAGED does not finalize
    // fn allocate_native_array<T>(
    //     collector: &GarbageCollector,
    //     len: usize,
    // ) -> std::result::Result<GcPtr<[MaybeUninit<T>]>, AllocationError> {
    //     let layout = Layout::array::<T>(len).map_err(AllocationError::LayoutError)?;

    //     let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();
    //     let root = GcRoot::new(ptr, len, layout, Mark::White, None);
    //     let mut pinned = Box::pin(root);
    //     let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
    //     collector.0.borrow_mut().objects.push(pinned);
    //     Ok(GcPtr::new(
    //         pinned_ptr,
    //         collector.0.borrow().collection_index,
    //         collector.0.borrow().collector_id,
    //     ))
    // }

    pub fn allocate<T: GcObject + Sized>(
        &mut self,
        v: T,
    ) -> std::result::Result<GcPtr<T>, AllocationError> {
        let layout = Layout::new::<T>();

        let ptr = unsafe { self.allocator.alloc(layout) } as *mut ();
        unsafe { std::ptr::write(ptr as *mut T, v) };
        let root = GcRoot::new(ptr, 0, layout, Mark::White, T::vtable());
        let mut pinned = Box::pin(root);
        let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
        self.objects.push(pinned);
        Ok(GcPtr::new(
            pinned_ptr,
            self.collection_index,
            self.collector_id,
        ))
    }

    pub fn visit_with<F: FnMut(&mut VisitorTy)>(jvm: JVM, mut f: F) {
        let mut visitor = PtrVisitor;
        f(&mut visitor);

        let gc = jvm.gc();
        let mut finalization_list = Vec::new();
        {
            let mut collector = gc.0.lock();
            let remove_list = Self::calc_remove_list(&mut collector);
            for idx in remove_list {
                let object = &mut collector.objects[idx];
                let finalizer = object.vtable.finalizer;
                let ptr: NonNullGcPtr<()> = GcPtr::new(
                    NonNull::new(&mut **object).unwrap(),
                    collector.collection_index,
                    collector.collector_id,
                )
                .promote()
                .unwrap();
                finalization_list.push((finalizer, ptr));
            }
        }
        {
            for (finalizer, ptr) in finalization_list {
                unsafe {
                    finalizer(ptr, jvm.new_ref());
                }
            }
        }
        {
            f(&mut visitor);
            let mut collector = gc.0.lock();
            collector.collection_index += 1;
            let new_remove_list = Self::calc_remove_list(&mut collector);

            for v in &new_remove_list {
                {
                    let object = &collector.objects[*v];
                    let ptr = object.ptr as *mut u8;
                    let layout = object.layout;
                    (object.vtable.dropper)(ptr as *mut (), object.meta);

                    unsafe {
                        collector.allocator.dealloc(ptr, layout);
                    }
                }
            }
            let mut new_list = Vec::with_capacity(collector.objects.capacity());
            for (idx, v) in std::mem::take(&mut collector.objects)
                .into_iter()
                .enumerate()
            {
                if !new_remove_list.contains(&idx) {
                    new_list.push(v);
                }
            }

            collector.objects = new_list;
        }
    }



    // fn allocate_array<T>(
    //     collector: &GarbageCollector,
    //     v: &[T],
    // ) -> std::result::Result<GcPtr<[T]>, AllocationError> {
    //     let layout = Layout::array::<T>(v.len()).unwrap();

    //     let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();
    //     unsafe {
    //         std::ptr::copy(v.as_ptr(), ptr as *mut T, v.len());
    //     };
    //     let root = GcRoot::new(ptr, v.len(), layout, Mark::White);
    //     let mut pinned = Box::pin(root);
    //     let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
    //     collector.0.borrow_mut().objects.push(pinned);
    //     Ok(GcPtr::new(
    //         pinned_ptr,
    //         collector.0.borrow().collection_index,
    //         collector.0.borrow().collector_id,
    //     ))
    // }

    pub fn collection_index(collector: &GarbageCollector) -> u8 {
        collector.0.borrow().collection_index
    }


    pub fn collector_id(collector: &GarbageCollector) -> u8 {
        collector.0.borrow().collector_id
    }

    pub fn new_global_ref<T: ?Sized>(
        collector: &GarbageCollector,
        v: GcPtr<T>,
    ) -> std::result::Result<OwnedPtr<T>, AllocationError> {
        v.ensure_same_collector(collector);
        let mut pinned = Box::pin(GlobalObject {
            ref_count: Cell::new(1),
            object: v.ptr(),
        });
        let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
        collector.0.borrow_mut().global_objects.push(pinned);
        Ok(OwnedGcPtr::new(v, pinned_ptr))
    }

    pub fn allocate_dst<T: super::collector::GcObject + ?Sized>(
        collector: &GarbageCollector,
        excess_size: usize,
        meta: usize,
    ) -> std::result::Result<GcPtr<T>, AllocationError> {
        const {
            if !T::DST {
                panic!("Must be DST");
            }
            if !T::NULLABLE {
                panic!("DST must be nullable");
            }
        };
        if T::valid_dynamic_size(excess_size) {
            let layout =
                Layout::from_size_align(T::MIN_SIZE_ALIGN.0 + excess_size, T::MIN_SIZE_ALIGN.1)
                    .map_err(AllocationError::LayoutError)?;

            let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();
            let root = GcRoot::new(ptr, meta, layout, Mark::White, T::vtable());
            let mut pinned = Box::pin(root);
            let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
            collector.0.borrow_mut().objects.push(pinned);
            Ok(GcPtr::new(
                pinned_ptr,
                collector.0.borrow().collection_index,
                collector.0.borrow().collector_id,
            ))
        } else {
            Err(AllocationError::InvalidDynamicSize(excess_size))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem::{align_of, size_of},
        num::NonZeroUsize,
        ptr::Thin,
        sync::atomic::AtomicBool,
    };

    use exo_class_file::item::ids::{
        field::{ArrayType, BaseType, FieldType},
        UnqualifiedName,
    };

    use crate::{
        nugc::collector::{GcObject, TheGc},
        structure::{FieldDef, StructureBuilder},
        value::{
            types::{ArrayMember, ExactJavaType, FieldNameAndType, JInt},
            Cast,
        },
        vm::{JVMBuilder, JVM},
    };

    use super::{
        super::collector::{GarbageCollector, Visitor},
        GcMut, GcPtr, OwnedGcPtr, VisitorTy,
    };

    use super::ThisCollector;

    unsafe impl GcObject for i32 {
        const MIN_SIZE_ALIGN: (usize, usize) = (size_of::<i32>(), align_of::<i32>());

        const NULLABLE: bool = true;

        const DST: bool = false;

        fn valid_dynamic_size(size: usize) -> bool {
            false
        }

        fn trace(
            &mut self,
            gc: &GarbageCollector,
            visitor: &mut VisitorTy,
        ) {
        }

        fn finalize(this: super::NonNullGcPtr<Self>, j: JVM) {}
    }

    #[test]
    fn test_owned() {
        let jvm = JVMBuilder::new().build();
        let value = jvm.gc().allocate(420i32).unwrap();

        let value: OwnedGcPtr<i32> = value.cast(&jvm).unwrap();

        jvm.gc().visit_with(jvm.new_ref(), |_| {});
        assert_eq!(*value.get(&jvm.gc()).unwrap(), 420);
    }

    #[test]
    #[should_panic]
    fn test_freed() {
        let gc = JVMBuilder::new().build();
        let value = gc.gc().allocate(420i32).unwrap();

        gc.gc().visit_with(gc.new_ref(), |_| {});

        assert_eq!(*value.get(&gc.gc()).unwrap(), 420);
    }

    // #[test]
    // #[should_panic]
    // fn test_structure_invalid_size() {
    //     let mut builder = StructureBuilder::new();
    //     builder.insert_field(FieldDef::new_java(FieldNameAndType));

    //     let strct = builder.build();

    //     let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
    //     let gc = GarbageCollector::new(allocator);

    //     let structure = gc.allocate_structure(&strct).unwrap();
    //     let mut s = structure.get_mut(&gc);
    //     let _: &mut i16 = unsafe { s.interpret_field(&strct, "balls") }.unwrap();
    // }

    // #[test]
    // fn test_arr() {
    //     let gc = JVMBuilder::new().build();

    //     let mut structure = gc
    //         .gc()
    //         .allocate_array(
    //             gc.gc()
    //                 .allocate(ExactJavaType::Array(
    //                     ArrayMember::Primitive(BaseType::Int),
    //                     1,
    //                 ))
    //                 .unwrap(),
    //             1,
    //         )
    //         .unwrap();

    //     {
    //         let mut s = structure.get_mut(&gc.gc()).unwrap();

    //         let int_array = gc
    //             .gc()
    //             .allocate_array(gc.gc().allocate(ExactJavaType::Int).unwrap(), 1)
    //             .unwrap();

    //         {
    //             let mut s = int_array.get_mut(&gc.gc()).unwrap();
    //             s.set_primitive(&gc.gc(), 0, 420i32).unwrap();
    //         }
    //         s.set_array_value(&gc.gc(), 0, int_array).unwrap();
    //     }

    //     ThisCollector::visit_with(gc.new_ref(), |v| {
    //         v.visit(&gc.gc(), &mut structure);
    //     });

    //     {
    //         let mut s = structure.get_mut(&gc.gc()).unwrap();
    //         let int_array = s.get_array_value(&gc.gc(), 0).unwrap();
    //         {
    //             let mut s = int_array.get_mut(&gc.gc()).unwrap();
    //             assert_eq!(s.get_primitive::<JInt>(&gc.gc(), 0).unwrap(), 420i32);
    //         }
    //     }
    // }

    // #[test]
    // fn test_structure() {
    //     let mut builder = StructureBuilder::new();
    //     builder.insert_field(FieldDef::new_native::<i32>("balls".to_string()));

    //     let strct = builder.build();

    //     let gc = JVMBuilder::new().build();
    //     let strct = gc.gc().allocate(strct).unwrap();

    //     let mut structure = gc.gc().allocate_structure(strct).unwrap();

    //     {
    //         let mut s = structure.get_mut(&gc.gc()).unwrap();
    //         s.write_native_field(&gc.gc(), "balls", 420).unwrap();
    //     }

    //     ThisCollector::visit_with(gc.new_ref(), |v| {
    //         v.visit(&gc.gc(), &mut structure);
    //     });

    //     {
    //         let mut s = structure.get_mut(&gc.gc()).unwrap();
    //         let field: Option<GcMut<i32>> = unsafe { s.native_field(&gc.gc(), "balls") };
    //         assert_eq!(*field.unwrap(), 420);
    //     }

    //     ThisCollector::visit_with(gc.new_ref(), |v| {});
    // }

    // #[test]
    // fn test_traced_structure() {
    //     static VALUE: AtomicBool = AtomicBool::new(false);

    //     struct CoolStruct;
    //     impl Trace<TheGc> for CoolStruct {
    //         fn trace(
    //             &mut self,
    //             _gc: &GarbageCollector,
    //             _visitor: &mut VisitorTy,
    //         ) {
    //             VALUE.store(true, std::sync::atomic::Ordering::SeqCst);
    //         }
    //     }
    //     impl Finalize for CoolStruct {
    //         unsafe fn finalize(this: crate::nugc::implementation::NonNullGcPtr<Self>, j: JVM) {}
    //     }

    //     let mut builder = StructureBuilder::new();
    //     builder.insert_field(FieldDef::new_native_traced::<CoolStruct>(
    //         "balls".to_string(),
    //     ));

    //     let strct = builder.build();

    //     let gc = JVMBuilder::new().build();
    //     let strct = gc.gc().allocate(strct).unwrap();

    //     let mut structure = gc.gc().allocate_structure(strct).unwrap();

    //     {
    //         let mut s = structure.get_mut(&gc.gc()).unwrap();
    //         s.write_native_field(&gc.gc(), "balls", CoolStruct).unwrap();
    //     }

    //     ThisCollector::visit_with(gc.new_ref(), |v| {
    //         v.visit(&gc.gc(), &mut structure);
    //     });

    //     assert!(VALUE.load(std::sync::atomic::Ordering::SeqCst))
    // }

    struct ThingWithAPtr {
        ptr: GcPtr<i32>,
    }

    unsafe impl GcObject for ThingWithAPtr {
        const MIN_SIZE_ALIGN: (usize, usize) = (size_of::<Self>(), align_of::<Self>());
        const NULLABLE: bool = true;
        const DST: bool = false;
        fn valid_dynamic_size(size: usize) -> bool {
            false
        }

        fn finalize(this: super::NonNullGcPtr<Self>, j: JVM) {}

        fn trace(
            &mut self,
            gc: &GarbageCollector,
            visitor: &mut VisitorTy,
        ) {
            println!("Was muvva fn called");
            visitor.mark(gc, &mut self.ptr);
        }
    }

    // impl<C: MemoryManager> Trace<C> for ThingWithAPtr<C> {
    //     fn trace(&mut self, gc: &GarbageCollector<C>, visitor: &mut C::VisitorTy) {
    //         println!("Was muvva fn called");
    //         visitor.mark(gc, &mut self.ptr);
    //     }
    // }

    // impl<C: MemoryManager> Finalize for ThingWithAPtr<C> {
    //     unsafe fn finalize(this: super::NonNullGcPtr<Self>, j: JVM) {}
    // }

    #[test]
    fn test_trace() {
        type OurThingWithAPtr = ThingWithAPtr;

        let gc = JVMBuilder::new().build();
        let value = ThisCollector::allocate(&gc.gc(), 420i32).unwrap();
        let mut value_two =
            ThisCollector::allocate(&gc.gc(), OurThingWithAPtr { ptr: value }).unwrap();

        gc.gc().visit_with(gc.new_ref(), |v| {
            v.visit(&gc.gc(), &mut value_two);
        });

        assert_eq!(gc.gc().0.borrow().objects.len(), 2);
    }

    #[test]
    #[should_panic]
    fn test_borrow() {
        let gc = JVMBuilder::new().build();

        let value = gc.gc().allocate(420i32).unwrap();

        let _borrow_one = value.get_mut(&gc.gc());
        let _borrow_two = value.get(&gc.gc());
    }

    // type Ptr<T> = GcPtr<T>;

    // struct EpicVM {
    //     gc: GarbageCollector,
    //     stack: Vec<Ptr<i32>>,
    // }

    // impl GcObject for EpicVM {
    //     fn trace(
    //         &mut self,
    //         collector: &GarbageCollector,
    //         visitor: &mut <ThisCollector as MemoryManager>::VisitorTy,
    //     ) {
    //         for v in &mut self.stack {
    //             visitor.mark(collector, v);
    //         }
    //     }
    // }

    // pub enum Instruction {
    //     Push(i32),
    //     Pop,
    //     Add,
    //     Sub,
    // }

    // impl EpicVM {
    //     pub fn new(gc: GarbageCollector) -> Self {
    //         Self {
    //             stack: Vec::new(),
    //             gc,
    //         }
    //     }

    //     fn alloc_num(&mut self, v: i32) -> Ptr<i32> {
    //         let start = self.gc.0.borrow().objects.len();
    //         if start > 10 {
    //             // not enough space
    //             ThisCollector::visit_with(&self.gc.clone(), |visitor| {
    //                 visitor.visit_noref(&self.gc.clone(), self);
    //             });

    //             let end = self.gc.0.borrow().objects.len();

    //             println!("Reclaimed {} objects", start - end);
    //             if end > 10 {
    //                 panic!("out of memory!");
    //             }
    //         }
    //         ThisCollector::allocate(&self.gc, v).unwrap()
    //     }

    //     pub fn eval(&mut self, instructions: &[Instruction]) {
    //         for inst in instructions {
    //             match inst {
    //                 Instruction::Push(v) => {
    //                     let v = self.alloc_num(*v);
    //                     self.stack.push(v);
    //                 }
    //                 Instruction::Pop => {
    //                     self.stack.pop();
    //                 }
    //                 Instruction::Add => {
    //                     let v2 = self.stack.pop().unwrap();
    //                     let v1 = self.stack.pop().unwrap();

    //                     let v =
    //                         self.alloc_num(*v1.get(&self.gc).unwrap() + *v2.get(&self.gc).unwrap());
    //                     self.stack.push(v);
    //                 }
    //                 Instruction::Sub => {
    //                     let v2 = self.stack.pop().unwrap();
    //                     let v1 = self.stack.pop().unwrap();

    //                     let v =
    //                         self.alloc_num(*v1.get(&self.gc).unwrap() - *v2.get(&self.gc).unwrap());
    //                     self.stack.push(v);
    //                 }
    //             }
    //         }
    //     }
    // }

    // #[test]
    // fn test_vm() {
    //     let gc = JVMBuilder::new().build();
    //     let mut vm = EpicVTheGc::new(gc.gc());
    //     vm.eval(&[
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(1),
    //         Instruction::Pop,
    //         Instruction::Push(9),
    //         Instruction::Push(10),
    //         Instruction::Add,
    //         Instruction::Push(1),
    //         Instruction::Sub,
    //         Instruction::Push(1),
    //         Instruction::Add,
    //     ]);
    //     assert_eq!(*vm.stack.pop().unwrap().get(&vm.gc).unwrap(), 19);
    // }
}

use std::{marker::PhantomData, num::NonZeroU64, ptr::NonNull, sync::atomic::Ordering};

use crate::vm::{
    collector::{
        gc::VMGcState,
        object::{GcObject, Trace, VisitorImpl},
        GcRootMeta, GcRootMetaCopyable,
    },
    thread::ThreadLocalHandle,
};

pub struct GcRef<T: ?Sized + GcObject> {
    /// Pointer to root.
    /// First 8 bits are the collector ID.
    /// The next 8 bits are the collection cycle number.
    ptr: NonZeroU64,
    _m: PhantomData<T>,
}

impl<T: ?Sized + GcObject> GcRef<T> {
    const COLLECTOR_ID: u64 = 0b1111111100000000000000000000000000000000000000000000000000000000;

    const COLLECTION_INDEX: u64 =
        0b0000000011111111000000000000000000000000000000000000000000000000;

    const COLLECTOR_ID_SHIFT: u64 = 56;
    const COLLECTION_INDEX_SHIFT: u64 = 48;

    pub(in super::super) fn new(
        ptr: NonNull<GcRootMeta>,
        collection_index: u8,
        collector_id: u8,
    ) -> Self {
        let mut v = ptr.as_ptr() as u64;
        v &= !Self::COLLECTOR_ID;
        v &= !Self::COLLECTION_INDEX;

        v |= (collector_id as u64) << Self::COLLECTOR_ID_SHIFT;
        v |= (collection_index as u64) << Self::COLLECTION_INDEX_SHIFT;
        Self {
            ptr: unsafe { NonZeroU64::new_unchecked(v) },
            _m: PhantomData,
        }
    }

    pub fn update<R, F: FnOnce(&mut T) -> R>(&mut self, f: F) -> R
    where
        T: Sized,
    {
        unsafe {
            let root = self.root();
            let ptr = root.data_ptr_mut::<T>();
            let v = root.lock.lock();
            let return_v = f(ptr.as_mut().unwrap());
            drop(v);
            return return_v;
        }
    }

    fn ptr(&self) -> *mut GcRootMeta {
        let mut v = self.ptr.get();
        v &= !Self::COLLECTOR_ID;
        v &= !Self::COLLECTION_INDEX;
        v as *mut GcRootMeta
    }

    pub fn root_copy(&self, volatile: bool) -> GcRootMetaCopyable {
        let ordering = match volatile {
            true => Ordering::SeqCst,
            false => Ordering::Relaxed,
        };
        let v = unsafe { load_raw(self.ptr() as *const GcRootMetaCopyable, ordering) };
        v
    }

    pub fn root(&mut self) -> &mut GcRootMeta {
        unsafe { self.ptr().as_mut().unwrap() }
    }

    fn collector_id(&self) -> u8 {
        let mut v = self.ptr.get();
        v &= Self::COLLECTOR_ID;
        (v >> Self::COLLECTOR_ID_SHIFT) as u8
    }

    fn collection_index(&self) -> u8 {
        let mut v = self.ptr.get();
        v &= Self::COLLECTION_INDEX;
        (v >> Self::COLLECTION_INDEX_SHIFT) as u8
    }

    fn set_collection_index(&mut self, v: u8) {
        self.ptr = unsafe { NonZeroU64::new_unchecked(self.ptr.get() & !Self::COLLECTION_INDEX) };
        self.ptr |= (v as u64) << Self::COLLECTION_INDEX_SHIFT;
    }

    fn check_same_thread(&self, t: &ThreadLocalHandle<'_>) {
        if self.collection_index() < t.state().collection_index {
            panic!("collection index mismatch");
        }
        if self.collector_id() != t.state().collector_id {
            panic!("collector id mismatch");
        }
    }

}


pub(crate) unsafe fn load_raw<L: Copy>(ptr: *const L, ordering: Ordering) -> L {
    match ordering {
        Ordering::SeqCst => std::intrinsics::atomic_load_seqcst(ptr),
        Ordering::Relaxed => std::intrinsics::atomic_load_relaxed(ptr),
        v => panic!("invalid ordering: {:?}", v),
    }
}

pub(crate) unsafe fn store_raw<L: Copy>(ptr: *mut L, ordering: Ordering, v: L) {
    match ordering {
        Ordering::SeqCst => std::intrinsics::atomic_store_seqcst(ptr, v),
        Ordering::Relaxed => std::intrinsics::atomic_store_relaxed(ptr, v),
        v => panic!("invalid ordering: {:?}", v),
    }
}

impl<T: ?Sized + GcObject + Copy> GcRef<T> {
    pub fn load(&self, thread: &ThreadLocalHandle<'_>, volatile: bool) -> T {
        self.check_same_thread(thread);
        let ordering = match volatile {
            true => Ordering::SeqCst,
            false => Ordering::Relaxed,
        };
        unsafe {
            let offset = load_raw(self.ptr.get() as *const usize, ordering);
            let data = (self.ptr.get() as *const u8).add(offset) as *const T;
            load_raw(data, ordering)
        }
    }

    pub fn store(&self, thread: &ThreadLocalHandle<'_>, volatile: bool, v: T) {
        self.check_same_thread(thread);
        let ordering = match volatile {
            true => Ordering::SeqCst,
            false => Ordering::Relaxed,
        };
        unsafe {
            let offset = load_raw(self.ptr.get() as *const usize, ordering);
            let data = (self.ptr.get() as *const u8).add(offset) as *mut T;
            store_raw(data, ordering, v)
        }
    }
}

unsafe impl<T: ?Sized + GcObject> Trace for GcRef<T> {
    const NEEDS_TRACED: bool = true;

    fn trace(&mut self, gc: &mut VMGcState, visitor: &mut VisitorImpl) {
        visitor.visit(gc, self.root());
        self.set_collection_index(self.collection_index() + 1);
    }
}
unsafe impl<T: ?Sized + GcObject> GcObject for GcRef<T> {}

static_assertions::assert_eq_size!(GcRef<()>, u64);

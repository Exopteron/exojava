use std::{
    alloc::Layout, any::TypeId, collections::LinkedList, marker::PhantomData, num::NonZeroUsize, fmt::Debug, ptr::drop_in_place, pin::Pin, mem::size_of,
};

use crate::vm::object::{JVMValue, JVMRefObjectType};

use super::linked_list::LinkedListAllocator;

/// Garbage collector implementation.
pub struct GarbageCollector<State: 'static> {
    allocator: LinkedListAllocator,
    objects: Vec<Pin<Box<GcRoot<State>>>>,
    _p: PhantomData<State>,
}
pub enum ArrayInitializer<'a> {
    Value(JVMValue),
    Values(&'a [JVMValue])
}

impl<State> GarbageCollector<State> {
    pub fn new(size: NonZeroUsize) -> Self {
        Self {
            allocator: LinkedListAllocator::new(size),
            objects: Vec::new(),
            _p: PhantomData,
        }
    }

    /// Creates a new object managed by this collector.
    pub unsafe fn new_object<T: Trace>(
        &mut self,
        v: T,
        finalizer: Option<FinalizerFn<State>>,
    ) -> GcPtr<T, State> {
        let layout = Layout::new::<T>();
        let ptr = self.allocator.alloc(layout);
        std::ptr::write(ptr as *mut T, v);
        let mut root = Box::pin(GcRoot::new(ptr, layout, NonZeroUsize::new(layout.size()).unwrap(), 1, Mark::White, Some(Box::new(|state, root| {
            if let Some(f) = finalizer {
                (f)(state, root);
            }
            let v = root.ptr as *mut T;
            drop_in_place(v);
        }))));
        let ptr = GcPtr::new(&mut *root);
        self.objects.push(root);
        ptr
    }


    pub unsafe fn new_array(
        &mut self,
        len: usize,
        init: &ArrayInitializer,
        finalizer: Option<FinalizerFn<State>>,
    ) -> GcPtr<JVMValue, State> {
        let layout = Layout::array::<JVMValue>(len).unwrap();
        let ptr = self.allocator.alloc(layout);
        let v_ptr = ptr as *mut JVMValue;
        for i in 0..len {
            match init {
                ArrayInitializer::Value(iv) => v_ptr.add(i).write(*iv),
                ArrayInitializer::Values(arr) => v_ptr.add(i).write(arr[i]),
            }
        }
        let mut root = Box::pin(GcRoot::new(ptr, layout, NonZeroUsize::new(size_of::<JVMValue>()).unwrap(), len, Mark::White, Some(Box::new(|state, root| {
            if let Some(f) = finalizer {
                (f)(state, root);
            }
            let v = root.ptr as *mut JVMValue;
            drop_in_place(v);
        }))));
        let ptr = GcPtr::new(&mut *root);
        self.objects.push(root);
        ptr
    }


    /// Sweep phase.
    pub unsafe fn sweep(&mut self, s: &State) {
        let mut remove_list = Vec::new();
        for (idx, root) in self.objects.iter_mut().enumerate() {
            if root.mark == Mark::White {
                remove_list.push(idx);
            }
        }

        for v in &remove_list {
            {
                let mut object = &mut self.objects[*v];
                if let Some(f) = object.finalizer.take() {
                    (f)(s, &object);
                }
                self.allocator.dealloc(object.ptr, object.layout);
            }
        }
        let mut new_list = Vec::with_capacity(self.objects.capacity());
        for (idx, v) in self.objects.iter().enumerate() {
            if !remove_list.contains(&idx) {
                new_list.push(v);
            }
        }

        for root in self.objects.iter_mut() {
            root.mark = Mark::White;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::GarbageCollector;

    #[test]
    fn alloc_test() {

        unsafe {
            let mut allocator = GarbageCollector::<String>::new(NonZeroUsize::new(5000).unwrap());
            // let v = allocator.new_object("Balls".to_string(), None);
            // println!("V: {:?}", v);    
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mark {
    White,
    Black,
}

pub type FinalizerFn<State> = Box<dyn FnOnce(&State, &GcRoot<State>)>;
/// Root object.
pub struct GcRoot<State> {
    ptr: *mut u8,
    layout: Layout,
    object_size: NonZeroUsize,
    object_count: usize,
    mark: Mark,
    finalizer: Option<FinalizerFn<State>>,
}

impl<State> GcRoot<State> {
    pub fn new(
        ptr: *mut u8,
        layout: Layout,
        object_size: NonZeroUsize,
        object_count: usize,
        mark: Mark,
        finalizer: Option<FinalizerFn<State>>,
    ) -> Self {
        Self {
            ptr,
            layout,
            mark,
            finalizer,
            object_size,
            object_count
        }
    }
}

/// Garbage-collected reference to object.
pub struct GcPtr<T, State> {
    object: *mut GcRoot<State>,
    _m: PhantomData<T>,
}

pub trait Trace {
    unsafe fn trace(&self);
}

impl<T: Trace, State> Debug for GcPtr<T, State> where T: Debug + Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("GcPtr").field("object", self.get_ref(0)).finish()
        }
    }
}

impl<T, State> Clone for GcPtr<T, State> {
    fn clone(&self) -> Self {
        Self {
            object: self.object,
            _m: PhantomData,
        }
    }
}
impl<T, State> Copy for GcPtr<T, State> {}

impl<T: Trace, State> GcPtr<T, State> {
    pub unsafe fn trace(&self) {
        if self.get_object().mark == Mark::White {
            self.get_object().mark = Mark::Black;
            for obj in self.get_ref_slice() {
                T::trace(obj);
            }
        }
    }
}

impl<T, State> GcPtr<T, State> {
    fn new(object: *mut GcRoot<State>) -> Self {
        Self {
            object,
            _m: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self { object: std::ptr::null_mut(), _m: PhantomData }
    }

    /// Check if two pointers point to the same object.
    pub fn ptr_eq(&self, other: GcPtr<T, State>) -> bool {
        std::ptr::eq(self.object, other.object)
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn get_object(&self) -> &mut GcRoot<State> {
        self.object.as_mut().unwrap()
    }

    pub fn len(&self) -> usize {
        unsafe{self.get_object()}.object_count
    }


    /// Get the object behind this pointer.
    ///
    /// Super unsafe! Will do bad things if
    /// you run this on a collected object!
    pub unsafe fn get(&mut self, index: usize) -> &mut T {
        assert!(index < self.get_object().object_count);
        (self.get_object().ptr as *mut T).add(index).as_mut().unwrap()
    }

    pub unsafe fn get_ref(&self, index: usize) -> &T {
        assert!(index < self.get_object().object_count);
        (self.get_object().ptr as *const T).add(index).as_ref().unwrap()
    }

    pub unsafe fn get_ref_slice(&self) -> &[T] {
        std::slice::from_raw_parts(self.get_object().ptr as *const T, self.get_object().object_count)
    }

    /// Mark this pointer as reachable for a sweep.
    pub unsafe fn mark_reachable(&mut self) {
        self.get_object().mark = Mark::Black;
    }

    /// Grab a non-generic version of this pointer.
    pub unsafe fn non_generic(self) -> NonGenericGcPtr<State>
    where
        T: 'static,
    {
        NonGenericGcPtr::new::<T>(self.object)
    }
}

/// Non-generic garbage collected pointer.
pub struct NonGenericGcPtr<State> {
    object: *mut GcRoot<State>,
    ty: TypeId,
}
impl<State> NonGenericGcPtr<State> {
    fn new<T: 'static>(object: *mut GcRoot<State>) -> Self {
        Self {
            object,
            ty: TypeId::of::<T>(),
        }
    }

    /// Convert to generic pointer.
    pub unsafe fn generic<T: 'static + Trace>(&mut self) -> GcPtr<T, State> {
        if TypeId::of::<T>() == self.ty {
            GcPtr::new(self.object)
        } else {
            panic!("Wrong type")
        }
    }

}

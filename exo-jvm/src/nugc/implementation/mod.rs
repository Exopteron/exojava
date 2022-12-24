use std::{
    alloc::Layout,
    cell::Cell,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{NonNull, Pointee},
    sync::atomic::{AtomicU32, Ordering},
};

use super::collector::{AllocationError, GarbageCollector, MemoryManager, Trace, Visitor};

use self::linked_list::LinkedListAllocator;

mod linked_list;

pub struct ThisCollector {
    allocator: LinkedListAllocator,
    objects: Vec<Pin<Box<GcRoot>>>,
    collection_index: usize,
    collector_id: u32,
}

impl ThisCollector {
    pub fn new(size: NonZeroUsize) -> Self {
        static COLLECTOR_ID: AtomicU32 = AtomicU32::new(0);
        Self {
            allocator: LinkedListAllocator::new(size),
            objects: Vec::new(),
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
pub struct GcRoot {
    ptr: *mut (),
    borrow_flag: Cell<BorrowFlag>,
    meta: usize,
    layout: Layout,
    mark: Mark,
}

impl GcRoot {
    pub fn new(ptr: *mut (), meta: usize, layout: Layout, mark: Mark) -> Self {
        Self {
            ptr,
            borrow_flag: Cell::new(UNUSED),
            layout,
            mark,
            meta,
        }
    }
}

pub struct GcRef<'a, T: ?Sized> {
    ptr: GcPtr<T>,
    r: &'a T,
}

impl<'a, T: ?Sized> Drop for GcRef<'a, T> {
    fn drop(&mut self) {
        unsafe { self.ptr.get_root_mut() }
            .borrow_flag
            .update(|v| v + 1);
    }
}

impl<'a, T: ?Sized> Deref for GcRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.r
    }
}

pub struct GcMut<'a, T: ?Sized> {
    ptr: GcPtr<T>,
    r: &'a mut T,
}

impl<'a, T: ?Sized> Drop for GcMut<'a, T> {
    fn drop(&mut self) {
        unsafe { self.ptr.get_root_mut() }
            .borrow_flag
            .update(|v| v - 1);
    }
}

impl<'a, T: ?Sized> Deref for GcMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.r
    }
}

impl<'a, T: ?Sized> DerefMut for GcMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.r
    }
}

pub struct PtrVisitor;

impl Visitor<ThisCollector> for PtrVisitor {
    fn visit<T: ?Sized + Trace<ThisCollector>>(
        &mut self,
        collector: &GarbageCollector<ThisCollector>,
        object: &mut <ThisCollector as MemoryManager>::Ptr<T>,
    ) {
        if self.mark(collector, object) {
            let mut value = object.get_mut(collector);
            value.trace(collector, self);
        }
    }

    fn visit_noref<T: ?Sized + Trace<ThisCollector>>(
        &mut self,
        collector: &GarbageCollector<ThisCollector>,
        object: &mut T,
    ) {
        object.trace(collector, self);
    }

    fn mark<T: ?Sized>(
        &mut self,
        _collector: &GarbageCollector<ThisCollector>,
        object: &mut <ThisCollector as MemoryManager>::Ptr<T>,
    ) -> bool {
        let ptr = unsafe { object.get_root_mut() };
        if ptr.mark == Mark::White {
            ptr.mark = Mark::Black;
            object.collection_index += 1;
            return true; // was not marked reachable
        }
        false // was reachable
    }
}

/// Garbage-collected reference to object.

pub struct GcPtr<T: ?Sized> {
    ptr: NonNull<GcRoot>,
    collection_index: usize,
    collector_id: u32,
    _m: PhantomData<T>,
}

impl<T: ?Sized> GcPtr<T> {
    fn new(ptr: NonNull<GcRoot>, collection_index: usize, collector_id: u32) -> Self {
        Self {
            ptr,
            collection_index,
            collector_id,
            _m: PhantomData,
        }
    }

    unsafe fn get_root(&self) -> &GcRoot {
        self.ptr.as_ref()
    }

    unsafe fn get_root_mut(&mut self) -> &mut GcRoot {
        self.ptr.as_mut()
    }

    pub fn ptr_eq(&self, other: GcPtr<T>) -> bool {
        std::ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }

    fn ensure_same_collector<M: MemoryManager>(&self, c: &GarbageCollector<M>) {
        if self.collector_id != M::collector_id(c) {
            panic!(
                "mismatched collector id (wrong collector passed in): {} {}",
                self.collector_id,
                M::collector_id(c)
            );
        }
        if self.collection_index < M::collection_index(c) {
            panic!(
                "mismatched collection index (potential use-after-free): {} {}",
                self.collection_index,
                M::collection_index(c)
            );
        }
    }

    pub fn get<M: MemoryManager>(&self, collector: &GarbageCollector<M>) -> GcRef<'_, T> {
        self.ensure_same_collector(collector);
        let root = unsafe { self.get_root() };
        if is_writing(root.borrow_flag.get()) {
            panic!("mutably borrowed");
        }
        root.borrow_flag.update(|v| v - 1);
        let meta =
            unsafe { std::mem::transmute_copy::<usize, <T as Pointee>::Metadata>(&root.meta) };
        let ptr = std::ptr::from_raw_parts(root.ptr, meta);
        unsafe {
            GcRef {
                ptr: *self,
                r: &*(ptr),
            }
        }
    }

    pub fn get_mut<M: MemoryManager>(&self, collector: &GarbageCollector<M>) -> GcMut<'_, T> {
        self.ensure_same_collector(collector);
        let root = unsafe { self.get_root() };
        if is_reading(root.borrow_flag.get()) {
            panic!("immutably borrowed");
        }
        root.borrow_flag.update(|v| v + 1);

        let meta =
            unsafe { std::mem::transmute_copy::<usize, <T as Pointee>::Metadata>(&root.meta) };
        let ptr = std::ptr::from_raw_parts_mut(root.ptr, meta);
        unsafe {
            GcMut {
                ptr: *self,
                r: &mut *(ptr),
            }
        }
    }
}

impl<T: ?Sized> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            collection_index: self.collection_index,
            collector_id: self.collector_id,
            _m: self._m,
        }
    }
}
impl<T: ?Sized> Copy for GcPtr<T> {}

impl ThisCollector {
    fn collect(collector: &GarbageCollector<Self>) {
        let mut collector = collector.0.borrow_mut();
        collector.collection_index += 1;
        let mut remove_list = Vec::new();
        for (idx, root) in collector.objects.iter_mut().enumerate() {
            if root.mark == Mark::White {
                remove_list.push(idx);
            }
        }

        for v in &remove_list {
            {
                let object = &collector.objects[*v];
                let ptr = object.ptr as *mut u8;
                let layout = object.layout;
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
            if !remove_list.contains(&idx) {
                new_list.push(v);
            }
        }

        collector.objects = new_list;

        for root in collector.objects.iter_mut() {
            root.mark = Mark::White;
        }
    }
}

impl MemoryManager for ThisCollector {
    type Ptr<T: ?Sized> = GcPtr<T>;

    type VisitorTy = PtrVisitor;

    fn allocate<T>(
        collector: &super::collector::GarbageCollector<Self>,
        v: T,
    ) -> std::result::Result<Self::Ptr<T>, AllocationError> {
        let layout = Layout::new::<T>();

        let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();
        unsafe { std::ptr::write(ptr as *mut T, v) };
        let root = GcRoot::new(ptr, 0, layout, Mark::White);
        let mut pinned = Box::pin(root);
        let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
        collector.0.borrow_mut().objects.push(pinned);
        Ok(GcPtr::new(
            pinned_ptr,
            collector.0.borrow().collection_index,
            collector.0.borrow().collector_id,
        ))
    }

    fn visit_with<F: FnOnce(&mut Self::VisitorTy)>(
        collector: &super::collector::GarbageCollector<Self>,
        f: F,
    ) {
        let mut visitor = PtrVisitor;
        f(&mut visitor);
        Self::collect(collector)
    }


    fn allocate_array<T>(
        collector: &GarbageCollector<Self>,
        v: &[T],
    ) -> std::result::Result<Self::Ptr<[T]>, AllocationError> {
        let layout = Layout::array::<T>(v.len()).unwrap();

        let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();
        unsafe {
            std::ptr::copy(v.as_ptr(), ptr as *mut T, v.len());
        };
        let root = GcRoot::new(ptr, v.len(), layout, Mark::White);
        let mut pinned = Box::pin(root);
        let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
        collector.0.borrow_mut().objects.push(pinned);
        Ok(GcPtr::new(
            pinned_ptr,
            collector.0.borrow().collection_index,
            collector.0.borrow().collector_id,
        ))
    }

    fn collection_index(collector: &GarbageCollector<Self>) -> usize {
        collector.0.borrow().collection_index
    }

    fn allocate_structure(
        collector: &GarbageCollector<Self>,
        structure: &crate::structure::StructureDef,
    ) -> std::result::Result<Self::Ptr<super::collector::Structure>, AllocationError> {
        let layout = Layout::from_size_align(structure.size(), structure.align())
            .map_err(AllocationError::LayoutError)?;

        let ptr = unsafe { collector.0.borrow_mut().allocator.alloc(layout) } as *mut ();

        unsafe {
            std::ptr::write_bytes(ptr as *mut u8, 0, structure.size());
        };

        let root = GcRoot::new(ptr, structure.size(), layout, Mark::White);
        let mut pinned = Box::pin(root);
        let pinned_ptr = NonNull::new(&mut *pinned).unwrap();
        collector.0.borrow_mut().objects.push(pinned);
        Ok(GcPtr::new(
            pinned_ptr,
            collector.0.borrow().collection_index,
            collector.0.borrow().collector_id,
        ))
    }

    fn collector_id(collector: &GarbageCollector<Self>) -> u32 {
        collector.0.borrow().collector_id
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize};

    use crate::structure::{FieldDef, StructureBuilder};

    use super::super::{
        collector::{GarbageCollector, MemoryManager, Trace, Visitor},
    };

    use super::{ThisCollector};

    #[test]
    #[should_panic]
    fn test_freed() {
        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);
        let value = gc.allocate(420i32).unwrap();

        gc.visit_with(|_| {});

        assert_eq!(*value.get(&gc), 420);
    }


    #[test]
    #[should_panic]
    fn test_structure_invalid_size() {
        let mut builder = StructureBuilder::new();
        builder.insert_field(FieldDef::new("balls".to_string(), 4, 4));


        let strct = builder.build();

        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);

        let structure = gc.allocate_structure(&strct).unwrap();
        let mut s = structure.get_mut(&gc);
        let _: &mut i16 = unsafe { s.interpret_field(&strct, "balls") }.unwrap();
    }

    #[test]
    fn test_structure() {
        let mut builder = StructureBuilder::new();
        builder.insert_field(FieldDef::new("balls_1".to_string(), 4, 4));

        builder.insert_field(FieldDef::new("balls_2".to_string(), 8, 8));


        let strct = builder.build();

        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);

        let mut structure = gc.allocate_structure(&strct).unwrap();

        {
            let mut s = structure.get_mut(&gc);
            let v: &mut i32 = unsafe { s.interpret_field(&strct, "balls_1") }.unwrap();
            *v = 420;

            let v: &mut i64 = unsafe { s.interpret_field(&strct, "balls_2") }.unwrap();
            *v = 69;
        }

        ThisCollector::visit_with(&gc, |v| {
            v.mark(&gc, &mut structure);
        });


        {
            let mut s = structure.get_mut(&gc);
            let v: &mut i32 = unsafe { s.interpret_field(&strct, "balls_1") }.unwrap();
            assert_eq!(*v, 420);

            let v: &mut i64 = unsafe { s.interpret_field(&strct, "balls_2") }.unwrap();
            assert_eq!(*v, 69);
        }
    }


    struct ThingWithAPtr<C: MemoryManager> {
        ptr: C::Ptr<i32>,
    }

    impl<C: MemoryManager> Trace<C> for ThingWithAPtr<C> {
        fn trace(&mut self, gc: &GarbageCollector<C>, visitor: &mut C::VisitorTy) {
            println!("Was muvva fn called");
            visitor.mark(gc, &mut self.ptr);
        }
    }

    #[test]
    fn test_trace() {
        type OurThingWithAPtr<'a> = ThingWithAPtr<ThisCollector>;

        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);
        let value = ThisCollector::allocate(&gc, 420i32).unwrap();
        let mut value_two = ThisCollector::allocate(&gc, OurThingWithAPtr { ptr: value }).unwrap();

        ThisCollector::visit_with(&gc, |v| {
            v.visit(&gc.clone(), &mut value_two);
        });

        

        assert_eq!(gc.0.borrow().objects.len(), 2);
    }

    #[test]
    #[should_panic]
    fn test_borrow() {
        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);
        let value = ThisCollector::allocate(&gc, 420i32).unwrap();

        let _borrow_one = value.get_mut(&gc);
        let _borrow_two = value.get(&gc);
    }

    type Ptr<T> = <ThisCollector as MemoryManager>::Ptr<T>;

    struct EpicVM {
        gc: GarbageCollector<ThisCollector>,
        stack: Vec<Ptr<i32>>,
    }

    impl Trace<ThisCollector> for EpicVM {
        fn trace(
            &mut self,
            collector: &GarbageCollector<ThisCollector>,
            visitor: &mut <ThisCollector as MemoryManager>::VisitorTy,
        ) {
            for v in &mut self.stack {
                visitor.mark(collector, v);
            }
        }
    }

    pub enum Instruction {
        Push(i32),
        Pop,
        Add,
        Sub,
    }

    impl EpicVM {
        pub fn new(gc: GarbageCollector<ThisCollector>) -> Self {
            Self {
                stack: Vec::new(),
                gc,
            }
        }

        fn alloc_num(&mut self, v: i32) -> Ptr<i32> {
            let start = self.gc.0.borrow().objects.len();
            if start > 10 {
                // not enough space
                ThisCollector::visit_with(&self.gc.clone(), |visitor| {
                    visitor.visit_noref(&self.gc.clone(), self);
                });

                let end = self.gc.0.borrow().objects.len();

                println!("Reclaimed {} objects", start - end);
                if end > 10 {
                    panic!("out of memory!");
                }
            }
            ThisCollector::allocate(&self.gc, v).unwrap()
        }

        pub fn eval(&mut self, instructions: &[Instruction]) {
            for inst in instructions {
                match inst {
                    Instruction::Push(v) => {
                        let v = self.alloc_num(*v);
                        self.stack.push(v);
                    }
                    Instruction::Pop => {
                        self.stack.pop();
                    }
                    Instruction::Add => {
                        let v2 = self.stack.pop().unwrap();
                        let v1 = self.stack.pop().unwrap();

                        let v = self.alloc_num(*v1.get(&self.gc) + *v2.get(&self.gc));
                        self.stack.push(v);
                    }
                    Instruction::Sub => {
                        let v2 = self.stack.pop().unwrap();
                        let v1 = self.stack.pop().unwrap();

                        let v = self.alloc_num(*v1.get(&self.gc) - *v2.get(&self.gc));
                        self.stack.push(v);
                    }
                }
            }
        }
    }

    #[test]
    fn test_vm() {
        let allocator = ThisCollector::new(NonZeroUsize::new(1_000_000).unwrap());
        let gc = GarbageCollector::new(allocator);
        let mut vm = EpicVM::new(gc);
        vm.eval(&[
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(1),
            Instruction::Pop,
            Instruction::Push(9),
            Instruction::Push(10),
            Instruction::Add,
            Instruction::Push(1),
            Instruction::Sub,
            Instruction::Push(1),
            Instruction::Add
        ]);
        assert_eq!(*vm.stack.pop().unwrap().get(&vm.gc), 19);
    }
}

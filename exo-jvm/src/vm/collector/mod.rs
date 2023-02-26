pub mod structures;

use std::{
    alloc::{self, Layout, dealloc},
    mem::{self, size_of}, num::NonZeroUsize, ptr::NonNull,
};

use parking_lot::Mutex;

use self::{gc::VMGcState, object::{VisitorImpl, GcObject}, structures::GcRef};

use super::VM;

#[macro_use]
pub mod object;
pub mod gc;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Mark {
    White,
    Black
}
impl Default for Mark {
    fn default() -> Self {
        Mark::White
    }
}

#[derive(Default, Clone, Copy)]
pub struct GcRootList {
    next: Option<NonNull<GcRootMeta>>,
    prev: Option<NonNull<GcRootMeta>>
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GcRootMetaCopyable {
    data_offset: usize,
    mark: Mark,
    layout: Layout
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GcRootVTable {
    needs_traced: bool,
    tracer: fn(&mut VisitorImpl, &mut VMGcState, *mut ()),
    finalizer: fn(NonNull<GcRootMeta>, u8, u8, VM, &mut VMGcState),
    dropper: fn(NonNull<GcRootMeta>)
}
impl GcRootVTable {
    pub fn new<T: GcObject>() -> Self {
        Self {
            needs_traced: T::NEEDS_TRACED,
            dropper: |mut p| {
                unsafe {
                    std::ptr::drop_in_place(p.as_mut().data_ptr_mut::<T>())
                }
            },
            finalizer: |ptr, collection_index, collector_id, vm, gc| {
                let v = GcRef::new(ptr, collection_index, collector_id);
                T::finalize(v, vm, gc);
            },
            tracer: |visitor, state, meta| {
                unsafe {
                    let v: *mut T = meta as *mut T;
                    T::trace(v.as_mut().unwrap(), state, visitor)
                }
            }
        }
    }
}

#[repr(C)]
pub struct GcRootMeta {
    data_offset: usize,
    mark: Mark,
    layout: Layout,
    list: GcRootList,
    vtable: GcRootVTable,
    lock: Mutex<()>,
}

impl GcRootMeta {
    pub unsafe fn data_ptr<T>(&self) -> *const T {
        let off = self.data_offset;
        (self as *const Self as *const u8).add(off) as *const T
    }
    pub unsafe fn data_ptr_mut<T>(&mut self) -> *mut T {
        let off = self.data_offset;
        (self as *mut Self as *mut u8).add(off) as *mut T
    }
}
unsafe impl Send for LinkedListAllocator {
    
}

/// Linked list memory allocator.
pub struct LinkedListAllocator {
    heap: *mut u8,
    layout: Layout,
    pub head: LinkedListNode,
    pub object_head: Option<NonNull<GcRootMeta>>
}
fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr // addr already aligned
    } else {
        addr - remainder + align
    }
}
impl LinkedListAllocator {
    pub fn new(size: NonZeroUsize) -> Self {
        unsafe {
            let layout = Layout::array::<u8>(size.get()).unwrap();
            let heap = alloc::alloc(layout);
            let head = LinkedListNode::new(0);
            Self { heap, layout, head, object_head: None }.init()
        }
    }

    unsafe fn init(mut self) -> Self {
        self.add_free_region(self.heap as usize, self.layout.size());
        self
    }

    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed region is capable of holding ListNode
        assert_eq!(align_up(addr, mem::align_of::<LinkedListNode>()), addr);
        assert!(size >= mem::size_of::<LinkedListNode>());

        let mut node = LinkedListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut LinkedListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    unsafe fn alloc_from_region(
        region: &LinkedListNode,
        size: usize,
        align: usize,
    ) -> Option<usize> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size)?;

        if alloc_end > region.end_addr() {
            // allocation too small
            return None;
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < size_of::<LinkedListNode>() {
            // region too small to hold a list node
            // region needs to have enough excess space
            // to hold a list node
            return None;
        }

        Some(alloc_start)
    }

    unsafe fn find_region(
        &mut self,
        size: usize,
        align: usize,
    ) -> Option<(&'static mut LinkedListNode, usize)> {
        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            if let Some(alloc_start) = Self::alloc_from_region(region, size, align) {
                // region suitable for allocation -> remove node from list
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }
        None
    }

    pub unsafe fn alloc(&mut self, layout: Layout, vtable: GcRootVTable) -> Option<NonNull<GcRootMeta>> {
        let (size, align, off) = Self::size_align(layout);
        if let Some((region, alloc_start)) = self.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess = region.end_addr() - alloc_end;
            if excess > 0 {
                self.add_free_region(alloc_end, excess);
            }
            let ptr = alloc_start as *mut u8;
            let v_ptr = ptr as *mut GcRootMeta;
            (*v_ptr).data_offset = off;
            (*v_ptr).vtable = vtable;
            (*v_ptr).layout = layout;
            std::ptr::write(&mut (*v_ptr).lock, Mutex::new(()));

            let list = if let Some(v) = &mut self.object_head {
                let mut cache = *v;
                let us = NonNull::new(v_ptr).unwrap();
                *v = us;
                cache.as_mut().list.prev = Some(us);
                GcRootList {
                    prev: None,
                    next: Some(cache),
                }
            } else {
                self.object_head = Some(NonNull::new(v_ptr).unwrap());
                GcRootList {
                    prev: None,
                    next: None,
                }
            };
            (*v_ptr).list = list;

            Some(NonNull::new(std::ptr::from_raw_parts_mut(ptr as *mut (), ())).unwrap())
        } else {
            None
        }
    }

    pub unsafe fn dealloc(&mut self, mut ptr: NonNull<GcRootMeta>) {
        let layout = ptr.as_mut().layout;
        let (size, _, _) = Self::size_align(layout);

        let meta_ptr = ptr.as_ptr();
        if self.object_head.is_some() && self.object_head == NonNull::new(meta_ptr) {
            self.object_head = (*meta_ptr).list.next;
            if let Some(mut v) = self.object_head {
                v.as_mut().list.prev = None;
            }
        } else {
            let mut prev_v = None;
            if let Some(mut prev) = (*meta_ptr).list.prev {
                prev.as_mut().list.next = (*meta_ptr).list.next;
                prev_v = Some(prev);
            }
            if let Some(mut next) = (*meta_ptr).list.next {
                next.as_mut().list.prev = prev_v;
            }
        }

        self.add_free_region(meta_ptr as *const u8 as usize, size);
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<LinkedListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        
        let (layout, offset) = Layout::new::<GcRootMeta>().extend(layout).unwrap();

        let size = layout.size().max(mem::size_of::<LinkedListNode>());
        (size, layout.align(), offset)
    }
}

impl Drop for LinkedListAllocator {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.heap, self.layout)
        }
    }
}

pub struct LinkedListNode {
    size: usize,
    pub next: Option<&'static mut LinkedListNode>,
}

impl LinkedListNode {
    pub fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }

}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, alloc::Layout};

    use super::{LinkedListAllocator, Mark};

    #[test]
    fn alloc_test() {
        // let mut allocator = LinkedListAllocator::new(NonZeroUsize::new(1024).unwrap());
        // unsafe {
        //     for i in 0..10 {
        //         let mut v = allocator.alloc(Layout::new::<i32>()).unwrap();
        //         let ptr = v.as_mut().data_ptr();
        //         *ptr = 5;
        //         if i % 2 == 0 {
        //             allocator.dealloc(v, Layout::new::<i32>());
        //         }
        //     }
        //     let mut next = allocator.object_head;
        //     let mut n = 0;
        //     while let Some(mut v) = next {
        //         next = v.as_mut().list.next;
        //         assert_eq!(v.as_mut().mark, Mark::White);
        //         n += 1;
        //     }
        //     assert_eq!(n, 5);
        // }
    }
}

/*


    pub unsafe fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
        let size = layout.size();
        let align = layout.align();

        let mut node = Some(self.list);
        println!("Sizre: {}", (*self.list).size());
        while let Some(n) = node {
            if (*n).size >= (size + size_of::<LinkedListNode>()) {
                println!("Breaking");
                break;
            } else {
                node = (*n).next();
                println!("Checking next which is none? {} because size is {}", node.is_none(), (*n).size());
            }
        }

        if let Some(node) = node {
            let ptr = node as *mut u8;

            let next = if let Some(next) = (*node).next() {
                next
            } else {
                let new_node = LinkedListNode::new(self.layout.size() - ((ptr.add(size) as usize) - (self.heap as usize)), None, (*node).prev());
                let new_ptr = ptr.add(size) as *mut LinkedListNode;
                std::ptr::write_unaligned(new_ptr, new_node);
                new_ptr
            };

            if let Some(prev) = (*node).prev() {
                (*prev).set_next(Some(next));
            } else {
                self.list = next;
            }

            Some(ptr)
        } else {
            println!("Nun");
            None
        }
        // let node = node?;
        // let start = node.add(1) as *mut u8;

        // let end = start.add(size);

        // let new_node = LinkedListNode::new(self.layout.size() - ((end as usize) - (self.heap as usize)), None, Some(node));

        // std::ptr::write(end as *mut LinkedListNode, new_node);
        // (*node).set_next(Some(end as *mut LinkedListNode));
        // (*node).size = size;

        // Some(start)
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) -> Option<()> {
        let mut head = self.list;
        while let Some(n) = (*head).next() {
            head = n;
        }



        let new_node = LinkedListNode::new(layout.size() + size_of::<LinkedListNode>(), None, Some(head));
        let new_ptr = ptr as *mut LinkedListNode;
        std::ptr::write_unaligned(new_ptr, new_node);

        (*head).set_next(Some(new_ptr));
        // let node = (ptr as *mut LinkedListNode).sub(1);

        // let prev = if let Some(prev) = (*node).prev() {
        //     prev
        // } else {
        //     println!("No prev");
        //     node
        // };

        // let next = (*node).next();
        // println!("Setting {} to {}", (*prev).size(), if let Some(next) = next {
        //     (*next).size()
        // } else {
        //     0
        // });
        // (*prev).set_next(next);

        Some(())
    }
*/

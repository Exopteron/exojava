use std::{
    alloc::{self, Layout, dealloc},
    mem::{self, size_of}, num::NonZeroUsize,
};

/// Linked list memory allocator.
pub struct LinkedListAllocator {
    heap: *mut u8,
    layout: Layout,
    pub head: LinkedListNode,
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
            Self { heap, layout, head }.init()
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

    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::size_align(layout);
        if let Some((region, alloc_start)) = self.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess = region.end_addr() - alloc_end;
            if excess > 0 {
                self.add_free_region(alloc_end, excess);
            }
            alloc_start as *mut u8
        } else {
            std::ptr::null_mut()
        }
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _) = Self::size_align(layout);
        self.add_free_region(ptr as usize, size);
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<LinkedListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<LinkedListNode>());
        (size, layout.align())
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

    pub fn size(&self) -> usize {
        self.size
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

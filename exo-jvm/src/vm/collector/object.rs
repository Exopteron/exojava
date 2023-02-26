
use crate::vm::VM;

use super::{structures::GcRef, gc::VMGcState, GcRootMeta, Mark};

type GarbageCollector = ();
pub struct VisitorImpl;
impl VisitorImpl {
    pub fn visit(&mut self, collector: &mut VMGcState, object: &mut GcRootMeta) {
        if self.mark(collector, object) && object.vtable.needs_traced {
            unsafe {
                (object.vtable.tracer)(self, collector, object.data_ptr_mut::<()>());
            }
        }
    }

    pub fn visit_noref<T: ?Sized + GcObject>(
        &mut self,
        collector: &mut VMGcState,
        object: &mut T,
    ) {
        if T::NEEDS_TRACED {
            object.trace(collector, self);
        }
    }
}
impl VisitorImpl {
    fn mark(&mut self, _collector: &mut VMGcState, object: &mut GcRootMeta) -> bool {
        if object.mark == Mark::White {
            object.mark = Mark::Black;
            return true;
        }
        false // was reachable
    }
}

/// # Safety
/// MIN_SIZE_ALIGN must be the minimum valid size and alignment of any instance of this type.
pub unsafe trait Trace {
    const NEEDS_TRACED: bool;
    fn trace(
        &mut self,
        gc: &mut VMGcState,
        visitor: &mut VisitorImpl,
    );
    // fn finalize(this: NonNullGcPtr<Self>, j: JVM); todo
}
pub unsafe trait GcObject: Trace {
    fn finalize(_this: GcRef<Self>, _vm: VM, _gc: &mut VMGcState) {}
}


macro_rules! bare_impl {
    ($($ty:ty),*) => {
        
        $(
            unsafe impl Trace for $ty {
                const NEEDS_TRACED: bool = false;
                fn trace(
                        &mut self,
                        _gc: &mut crate::vm::VMGcState,
                        _visitor: &mut crate::vm::collector::VisitorImpl,
                    ) {
                    
                }
            }

            unsafe impl GcObject for $ty {

            }
        )*

    };
}
pub(crate) use bare_impl;

bare_impl!((), u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);
//! Object memory management/allocation.


mod linked_list;
mod gc;

pub use gc::{GarbageCollector, GcPtr, Trace, ArrayInitializer};
use std::num::NonZeroUsize;

pub mod types;



use crate::nugc::{implementation::ThisCollector, collector::GarbageCollector};


// temp
pub type JVM = GarbageCollector<ThisCollector>;

pub type JVMResult<T> = std::result::Result<T, ()>;


pub trait JavaType {
    fn size(&self) -> usize;
    fn align(&self) -> NonZeroUsize;
}  


/// Casting between Java objects.
pub trait Cast<Output>: JavaType {

    fn cast(self, j: JVM) -> JVMResult<Output>;
}
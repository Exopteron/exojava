// use std::num::NonZeroUsize;

// use crate::vm::JVM;

// pub mod types;



// // use crate::{nugc::{implementation::ThisCollector, collector::GarbageCollector}, vm::JVM};


// // temp
// pub type JVMResult<T> = std::result::Result<T, ()>;


// pub trait JavaType {
//     fn size(&self) -> usize;
//     fn align(&self) -> NonZeroUsize;
// }  


// /// Casting between Java objects.
// pub trait Cast<Output>: JavaType {

//     fn cast(self, j: &JVM) -> JVMResult<Output>;
// }
//! The core JVM implementation.
#![feature(specialization,arbitrary_self_types,  inline_const, cell_update, ptr_metadata, core_intrinsics, pointer_byte_offsets)]

pub use exo_class_file;
// pub mod structure;
pub mod nugc;
pub mod vm;

mod value;
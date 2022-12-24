//! The core JVM implementation.
#![feature(ptr_metadata)]
#![feature(cell_update)]

pub use exo_class_file;
pub mod structure;
pub mod nugc;

mod value;
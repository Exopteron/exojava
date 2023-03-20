use std::{io::Cursor, fs::File};

use afl::fuzz;
use exo_class_file::{item::{file::ClassFile, ClassFileItem}, stream::ClassFileStream};

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(v) = ClassFile::read_from_stream(&mut ClassFileStream::new(&mut Cursor::new(data)), None) {
            let _ = v.constant_pool.verify_cp_index_types();
            let _ = v.constant_pool.verify_structure(&v);
        }
    });
}

// #[test]
// fn epic_fuzz_test() {
//     if let Ok(v) = ClassFile::read_from_stream(&mut ClassFileStream::new(&mut File::open("").unwrap()), None) {
//         let _ = v.constant_pool.verify_cp_index_types();
//         let _ = v.constant_pool.verify_structure(&v);
//     }
// }
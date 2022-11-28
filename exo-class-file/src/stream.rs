use std::io::Read;

use crate::{error, item::{ClassFileItem, ConstantPool}};

/// A utility wrapper to allow easily reading class file types from a [Reader](std::io::Read).
pub struct ClassFileStream<'a, R: Read>(pub &'a mut R, pub usize);

impl<'a, R: Read> ClassFileStream<'a, R> {

    /// Create a new stream from a reader.
    pub fn new(r: &'a mut R) -> Self {
        Self(r, 0)
    }

    /// Read a sequence of `length` `T`s from this stream.
    pub fn read_sequence<T: ClassFileItem>(&mut self, constant_pool: Option<&ConstantPool>, length: usize) -> error::Result<Vec<T>> {
        let mut v = Vec::with_capacity(length);
        for _ in 0..length {
            v.push(T::read_from_stream(self, constant_pool)?);
        }
        Ok(v)
    }

    /// Read an unsigned 4-byte integer from the stream.
    pub fn read_u4(&mut self) -> error::Result<u32> {
        Ok(u32::from_be_bytes(self.read::<4>()?))
    }

    /// Read an unsigned 2-byte integer from the stream.
    pub fn read_u2(&mut self) -> error::Result<u16> {
        Ok(u16::from_be_bytes(self.read::<2>()?))
    }

    /// Read an unsigned byte from the stream.
    pub fn read_u1(&mut self) -> error::Result<u8> {
        Ok(self.read::<1>()?[0])
    }

    /// Utility method to read `S` bytes from the stream.
    pub fn read<const S: usize>(&mut self) -> error::Result<[u8; S]> {
        let mut w = [0; S];
        self.0
            .read_exact(&mut w)
            .map_err(error::ClassFileError::IoError)?;
        self.1 += S;
        Ok(w)
    }

    /// Utility method to read `S` bytes from the stream with runtime length.
    pub fn read_dynamic(&mut self, l: usize) -> error::Result<Vec<u8>> {
        let mut w = vec![0; l];
        self.0
            .read_exact(&mut w)
            .map_err(error::ClassFileError::IoError)?;
        self.1 += l;
        Ok(w)
    }

}
impl ClassFileItem for u8 {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: std::marker::Sized {
        s.read_u1()
    }
}

impl ClassFileItem for u16 {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: std::marker::Sized {
        s.read_u2()
    }
}

impl ClassFileItem for i16 {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: std::marker::Sized {
        Ok(s.read_u2()? as i16)
    }
}

impl ClassFileItem for u32 {
    fn read_from_stream<R: Read>(s: &mut ClassFileStream<R>, cp: Option<&ConstantPool>) -> error::Result<Self>
    where
        Self: std::marker::Sized {
        s.read_u4()
    }
}
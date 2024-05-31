use std::io::{Read, Write};

#[cfg(debug_assertions)]
pub mod mock;

pub trait UnsafeRead: Read {
    /// # Safety
    /// Function must be externally synchronized, calling function from two places same time will make undefinied behavior.
    unsafe fn read_unsafe(&self, buf: &mut [u8]) -> std::io::Result<usize>;
}

pub trait UnsafeWrite: Write {
    /// # Safety
    /// Function must be externally synchronized, calling function from two places same time will make undefinied behavior.
    unsafe fn write_unsafe(&self, buf: &[u8]) -> std::io::Result<usize>;

    /// # Safety
    /// Function must be externally synchronized, calling function from two places same time will make undefinied behavior.
    unsafe fn flush_unsafe(&self) -> std::io::Result<()>;
}

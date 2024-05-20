use std::{mem, slice, thread::ThreadId};

use wie_common::stream::{UnsafeRead, UnsafeWrite};

use crate::Connection;

#[repr(C)]
pub(crate) struct PacketHeader {
    pub length: usize,
    pub thread_id: Option<ThreadId>,
}

pub struct PacketWriter<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    connection: &'c Connection<T>,
    buffer: Vec<u8>,
}

impl<'c, T> PacketWriter<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    #[inline]
    pub(crate) fn new(connection: &'c Connection<T>, buffer: Vec<u8>) -> Self {
        Self { connection, buffer }
    }

    #[inline]
    pub fn write<TO>(&mut self, object: TO) {
        let slice = unsafe {
            slice::from_raw_parts(&object as *const _ as *const u8, mem::size_of::<TO>())
        };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn write_raw_ptr<TO>(&mut self, object: *const TO) {
        let slice = unsafe { slice::from_raw_parts(object as *const u8, mem::size_of::<TO>()) };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn send(mut self) {
        let buffer = mem::take(&mut self.buffer);
        self.connection.send(buffer)
    }

    #[inline]
    pub fn send_with_response(mut self) -> Packet<'c, T> {
        let buffer = mem::take(&mut self.buffer);
        self.connection.send_with_response(buffer)
    }
}

impl<T> Drop for PacketWriter<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    fn drop(&mut self) {
        // Ignore if buffer is cleared.
        if self.buffer.capacity() != 0 {
            let buffer = mem::take(&mut self.buffer);
            self.connection.push_buffer(buffer);
        }
    }
}

pub struct Packet<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    connection: &'c Connection<T>,
    buffer: Vec<u8>,
    read: usize,
}

impl<'c, T> Packet<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    pub(crate) fn new(connection: &'c Connection<T>, buffer: Vec<u8>) -> Self {
        Self {
            connection,
            buffer,
            read: 0,
        }
    }

    pub fn write_response() -> PacketWriter<'c, T> {
        todo!()
    }
}

impl<T> Drop for Packet<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    fn drop(&mut self) {
        // Ignore if buffer is cleared.
        if self.buffer.capacity() != 0 {
            let buffer = mem::take(&mut self.buffer);
            self.connection.push_buffer(buffer);
        }
    }
}

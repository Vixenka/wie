use std::{
    mem::{self, MaybeUninit},
    ptr, slice,
    thread::ThreadId,
};

use wie_common::stream::{UnsafeRead, UnsafeWrite};

use crate::Connection;

#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct PacketHeader {
    pub length: usize,
    pub sender_thread_id: Option<ThreadId>,
    pub destination: Destination,
}

#[derive(Clone, Debug)]
pub(crate) enum Destination {
    Thread(ThreadId),
    Handler(u64),
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
    pub(crate) fn new(
        connection: &'c Connection<T>,
        mut buffer: Vec<u8>,
        destination: Destination,
    ) -> Self {
        let header = unsafe { &mut *(buffer.as_mut_ptr() as *mut PacketHeader) };
        header.destination = destination;
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
        self.connection.send(buffer);
        mem::forget(self);
    }

    #[inline]
    pub fn send_with_response(mut self) -> Packet<'c, T> {
        let buffer = mem::take(&mut self.buffer);
        let packet = self.connection.send_with_response(buffer);
        mem::forget(self);
        packet
    }
}

impl<T> Drop for PacketWriter<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    fn drop(&mut self) {
        panic!("PacketWriter dropped without sending packet.")
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
            read: mem::size_of::<PacketHeader>(),
        }
    }

    pub(crate) fn header(&self) -> &PacketHeader {
        unsafe { &*(self.buffer.as_ptr() as *const PacketHeader) }
    }

    pub fn read<TO>(&mut self) -> TO {
        let size = mem::size_of::<TO>();

        let mut object = MaybeUninit::<TO>::uninit();
        unsafe {
            ptr::copy_nonoverlapping(
                self.buffer[self.read..].as_ptr(),
                &mut object as *mut _ as *mut u8,
                size,
            );
        }

        self.read += size;
        unsafe { object.assume_init() }
    }

    pub fn read_to_raw_ptr<TO>(&mut self, ptr: *mut TO) {
        let size = mem::size_of::<TO>();
        unsafe {
            ptr::copy_nonoverlapping(self.buffer[self.read..].as_ptr(), ptr as *mut u8, size);
        }
        self.read += size;
    }

    pub fn write_response(mut self, destination: Option<u64>) -> PacketWriter<'c, T> {
        if self.buffer.len() != self.read {
            panic!("Packet buffer is not fully read.");
        }

        let destination = match destination {
            Some(d) => Destination::Handler(d),
            None => match self.header().sender_thread_id {
                Some(thread_id) => Destination::Thread(thread_id),
                None => panic!("packet does not have sender thread id or destination is not set"),
            },
        };

        let mut buffer = mem::take(&mut self.buffer);
        unsafe { buffer.set_len(mem::size_of::<PacketHeader>()) };
        PacketWriter::new(self.connection, buffer, destination)
    }
}

impl<T> Drop for Packet<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    fn drop(&mut self) {
        // Ignore if buffer is cleared.
        if self.buffer.capacity() != 0 {
            if self.buffer.len() != self.read {
                panic!("Packet buffer is not fully read.");
            }

            let buffer = mem::take(&mut self.buffer);
            self.connection.push_buffer(buffer);
        }
    }
}

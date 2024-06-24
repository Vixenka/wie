use std::{
    ffi::c_char,
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
        self.align::<TO>();
        let slice = unsafe {
            slice::from_raw_parts(&object as *const _ as *const u8, mem::size_of::<TO>())
        };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn write_raw_ptr<TO>(&mut self, object: *const TO) {
        self.align::<TO>();
        let slice = unsafe { slice::from_raw_parts(object as *const u8, mem::size_of::<TO>()) };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn write_nullable_raw_ptr<TO>(&mut self, object: *const TO) {
        if object.is_null() {
            self.buffer.push(0);
        } else {
            self.buffer.push(1);
            self.write_raw_ptr(object);
        }
    }

    #[inline]
    pub fn write_nullable_raw_ptr_mut<TO>(&mut self, object: *mut TO) {
        self.write_nullable_raw_ptr(object as *const TO);
    }

    /// # Safety
    /// This function is unsafe because it writes until null character is found.
    #[inline]
    pub unsafe fn write_null_str(&mut self, str: *const c_char) {
        if str.is_null() {
            self.buffer.push(0);
            return;
        }

        let mut i = 0;
        while *str.add(i) != 0 {
            self.buffer.push(*str.add(i) as u8);
            i += 1;
        }

        self.buffer.push(0);
    }

    #[inline]
    pub fn write_is_null_ptr<TO>(&mut self, ptr: *const TO) {
        self.buffer.push(ptr.is_null().into())
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to count, and valid pointer or null to a buffer.
    #[inline]
    pub unsafe fn write_vk_array_count<TO>(&mut self, count: *const u32, buffer: *const TO) {
        self.write(match buffer.is_null() {
            true => 0,
            false => *count,
        });
    }

    #[inline]
    pub fn write_vk_array<TO>(&mut self, count: u32, buffer: *const TO) {
        self.write(count);
        if !buffer.is_null() {
            let slice = unsafe {
                slice::from_raw_parts(buffer as *mut u8, count as usize * mem::size_of::<TO>())
            };
            self.buffer.extend_from_slice(slice);
        }
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

    #[inline]
    fn align<TO>(&mut self) {
        let m = self.buffer.len() % mem::align_of::<TO>();
        if m == 0 {
            return;
        }

        let missing = mem::align_of::<TO>() - m;
        if self.buffer.capacity() < self.buffer.len() + missing {
            self.buffer
                .reserve(missing - (self.buffer.capacity() - self.buffer.len()));
        }
        unsafe {
            self.buffer.set_len(self.buffer.len() + missing);
        }

        debug_assert_eq!(0, self.buffer.len() % mem::align_of::<TO>());
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

    #[inline]
    pub fn read<TO>(&mut self) -> TO {
        self.align::<TO>();
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

    #[inline]
    pub fn read_to_raw_ptr<TO>(&mut self, ptr: *mut TO) {
        self.align::<TO>();
        let size = mem::size_of::<TO>();
        unsafe {
            ptr::copy_nonoverlapping(self.buffer[self.read..].as_ptr(), ptr as *mut u8, size);
        }
        self.read += size;
    }

    #[inline]
    pub fn read_nullable_raw_ptr<TO>(&mut self) -> *const TO {
        if self.buffer[self.read] == 0 {
            self.read += 1;
            ptr::null()
        } else {
            self.read += 1;
            self.align::<TO>();
            let ptr = unsafe { self.buffer.as_ptr().add(self.read) } as *const TO;
            self.read += mem::size_of::<TO>();
            ptr
        }
    }

    #[inline]
    pub fn read_nullable_raw_ptr_mut<TO>(&mut self) -> *mut TO {
        self.read_nullable_raw_ptr::<TO>() as *mut TO
    }

    #[inline]
    pub fn read_null_str(&mut self) -> *const c_char {
        if self.buffer[self.read] == 0 {
            self.read += 1;
            return ptr::null();
        }

        let start = self.read;
        while self.buffer[self.read] != 0 {
            self.read += 1;
        }

        self.read += 1;
        self.buffer[start..self.read].as_ptr() as *const c_char
    }

    #[inline]
    pub fn read_is_null_ptr(&mut self) -> bool {
        let is_null = self.buffer[self.read] == 1;
        self.read += 1;
        is_null
    }

    #[inline]
    pub fn read_and_allocate_vk_array_count<TA>(&mut self) -> (u32, *mut TA) {
        let count = self.read::<u32>();
        match count == 0 {
            true => (0, ptr::null_mut()),
            false => {
                // TODO: fix memleak here
                (
                    count,
                    Vec::with_capacity(count as usize).leak() as *mut [TA] as *mut TA,
                )
            }
        }
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to count, and valid pointer or null to a destination.
    #[inline]
    pub unsafe fn read_vk_array<TO>(&mut self, count: *mut u32, destination: *mut TO) {
        let c = self.read::<u32>();
        *count = c;
        if !destination.is_null() {
            ptr::copy_nonoverlapping(
                self.buffer[self.read..].as_ptr() as *const TO,
                destination,
                c as usize,
            );
            self.read += c as usize * mem::size_of::<TO>();
        }
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

    #[inline]
    fn align<TO>(&mut self) {
        let m = self.read % mem::align_of::<TO>();
        if m != 0 {
            self.read += mem::align_of::<TO>() - m;
        }
        debug_assert_eq!(0, self.read % mem::align_of::<TO>());
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

#[cfg(all(test, debug_assertions))]
mod tests {
    use std::{
        ffi::CStr,
        mem,
        ptr::{self, NonNull},
    };

    use wie_common::stream::mock::MockStream;

    use crate::packet::PacketHeader;

    use super::{Destination, Packet, PacketWriter};

    fn helper<F1, F2>(write: F1, read: F2)
    where
        F1: FnOnce(&mut PacketWriter<'_, MockStream>),
        F2: FnOnce(&mut Packet<'_, MockStream>),
    {
        let connection = unsafe { NonNull::dangling().as_ref() };

        let mut writer = PacketWriter::new(
            connection,
            vec![0; mem::size_of::<PacketHeader>()],
            Destination::Handler(0),
        );
        write(&mut writer);

        let mut packet = Packet::new(connection, mem::take(&mut writer.buffer));
        read(&mut packet);

        assert_eq!(packet.buffer.len(), packet.read);

        _ = mem::take(&mut packet.buffer);
        mem::forget(writer);
        mem::forget(packet);
    }

    #[test]
    fn write_read() {
        helper(
            |packet| {
                packet.write(11u8);
                packet.write(34.13f64);
            },
            |packet| {
                assert_eq!(packet.read::<u8>(), 11);
                assert_eq!(packet.read::<f64>(), 34.13);
            },
        )
    }

    #[test]
    fn vk_array_count_null() {
        helper(
            |packet| {
                let count = 1984u32;
                unsafe {
                    packet.write_vk_array_count(&count as *const _, ptr::null::<i128>());
                }
            },
            |packet| {
                let (count, buffer) = packet.read_and_allocate_vk_array_count::<i128>();
                assert_eq!(count, 0);
                assert_eq!(ptr::null(), buffer);
            },
        )
    }

    #[test]
    fn vk_array_count() {
        helper(
            |packet| {
                let count = 3u32;
                let buffer = [1, 2, 3];
                unsafe {
                    packet.write_vk_array_count(&count as *const _, buffer.as_ptr());
                }
            },
            |packet| {
                let (count, buffer) = packet.read_and_allocate_vk_array_count::<i128>();
                assert_eq!(count, 3);
                assert_ne!(ptr::null(), buffer);
            },
        )
    }

    #[test]
    fn vk_array() {
        helper(
            |packet| {
                let count = 3u32;
                let buffer = [1, 2, 3];
                packet.write_vk_array(count, buffer.as_ptr());
            },
            |packet| {
                let mut count = 0u32;
                let mut buffer = [0, 0, 0];

                unsafe { packet.read_vk_array(&mut count as *mut _, buffer.as_mut_ptr()) };
                assert_eq!(count, 3);
                assert_eq!([1, 2, 3], buffer);
            },
        )
    }

    #[test]
    fn raw_ptr() {
        helper(
            |packet| {
                let obj = 1984f64;
                packet.write_raw_ptr(&obj as *const _);
            },
            |packet| {
                let mut obj = 0f64;
                packet.read_to_raw_ptr(&mut obj as *mut _);
                assert_eq!(1984f64, obj);
            },
        )
    }

    #[test]
    fn nullable_raw_ptr() {
        helper(
            |packet| {
                let obj = 1984f64;
                packet.write_nullable_raw_ptr(&obj as *const _);
            },
            |packet| {
                let ptr = packet.read_nullable_raw_ptr::<f64>();
                assert_eq!(1984f64, unsafe { *ptr });
            },
        )
    }

    #[test]
    fn nullable_raw_ptr_mut() {
        helper(
            |packet| {
                let mut obj = 1984f64;
                packet.write_nullable_raw_ptr_mut(&mut obj as *mut _);
            },
            |packet| {
                let ptr = packet.read_nullable_raw_ptr::<f64>();
                assert_eq!(1984f64, unsafe { *ptr });
            },
        )
    }

    #[test]
    fn write_null_str() {
        let str = b"Hello world\0";
        helper(
            |packet| unsafe {
                packet.write_null_str(str.as_ptr() as *const i8);
            },
            |packet| {
                let read = unsafe { CStr::from_ptr(packet.read_null_str()) };
                assert_eq!(unsafe { CStr::from_ptr(str.as_ptr() as *const i8) }, read);
            },
        )
    }
}

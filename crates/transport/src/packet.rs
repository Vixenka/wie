use std::{
    cell::UnsafeCell,
    ffi::c_char,
    mem::{self, MaybeUninit},
    ptr, slice,
    thread::ThreadId,
};

use aligned_vec::AVec;
use cdump::{CDeserialize, CDumpReader, CDumpWriter, CSerialize};
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
    buffer: AVec<u8>,
    /// Store read buffer to extend lifetime of read variables.
    read_buffer: AVec<u8>,
}

impl<'c, T> PacketWriter<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    #[inline]
    pub(crate) fn new(
        connection: &'c Connection<T>,
        mut buffer: AVec<u8>,
        read_buffer: AVec<u8>,
        destination: Destination,
    ) -> Self {
        let header = unsafe { &mut *(buffer.as_mut_ptr() as *mut PacketHeader) };
        header.destination = destination;
        Self {
            connection,
            buffer,
            read_buffer,
        }
    }

    #[inline]
    pub fn write_shallow<TO>(&mut self, object: TO) {
        self.align::<TO>();
        let slice = unsafe {
            slice::from_raw_parts(&object as *const _ as *const u8, mem::size_of::<TO>())
        };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn write_raw_ptr_as_shallow<TO>(&mut self, object: *const TO) {
        self.align::<TO>();
        let slice = unsafe { slice::from_raw_parts(object as *const u8, mem::size_of::<TO>()) };
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    pub fn write_shallow_under_nullable_ptr<TO>(&mut self, object: *const TO) {
        if object.is_null() {
            self.buffer.push(0);
        } else {
            self.buffer.push(1);
            self.write_raw_ptr_as_shallow(object);
        }
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to object or null.
    #[inline]
    pub unsafe fn write_deep<TO>(&mut self, object: *const TO)
    where
        TO: CSerialize<PacketWriter<'c, T>>,
    {
        if object.is_null() {
            self.buffer.push(0);
        } else {
            self.buffer.push(1);
            let object_ref = &*object;
            object_ref.serialize(self);
        }
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to object or null.
    #[inline]
    pub unsafe fn write_deep_double<TO>(&mut self, _object: *const *const TO)
    where
        TO: CSerialize<PacketWriter<'c, T>>,
    {
        todo!()
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
        self.write_shallow(match buffer.is_null() {
            true => 0,
            false => *count,
        });
    }

    #[inline]
    pub fn write_vk_array<TO>(&mut self, count: u32, buffer: *const TO) {
        self.write_shallow(count);
        if !buffer.is_null() {
            let slice = unsafe {
                slice::from_raw_parts(buffer as *mut u8, count as usize * mem::size_of::<TO>())
            };
            self.buffer.extend_from_slice(slice);
        }
    }

    #[inline]
    pub fn send(mut self) {
        let buffer = mem::replace(&mut self.buffer, AVec::with_capacity(0, 0));
        self.connection.send(buffer);
        self.connection.push_buffer(mem::replace(
            &mut self.read_buffer,
            AVec::with_capacity(0, 0),
        ));
        mem::forget(self);
    }

    #[inline]
    pub fn send_with_response(mut self) -> Packet<'c, T> {
        let buffer = mem::replace(&mut self.buffer, AVec::with_capacity(0, 0));
        let packet = self.connection.send_with_response(buffer);
        self.connection.push_buffer(mem::replace(
            &mut self.read_buffer,
            AVec::with_capacity(0, 0),
        ));
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

unsafe impl<T> CDumpWriter for PacketWriter<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    #[inline]
    fn align<TO>(&mut self) {
        self.align::<TO>();
    }

    #[inline]
    fn push_slice(&mut self, slice: &[u8]) {
        self.buffer.extend_from_slice(slice);
    }

    #[inline]
    fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    unsafe fn as_mut_ptr_at(&mut self, index: usize) -> *mut u8 {
        self.buffer.as_mut_ptr().add(index)
    }
}

pub struct Packet<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    connection: &'c Connection<T>,
    buffer: UnsafeCell<AVec<u8>>,
    read: usize,
}

impl<'c, T> Packet<'c, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    pub(crate) fn new(connection: &'c Connection<T>, buffer: AVec<u8>) -> Self {
        Self {
            connection,
            buffer: UnsafeCell::new(buffer),
            read: mem::size_of::<PacketHeader>(),
        }
    }

    pub(crate) fn header(&self) -> &PacketHeader {
        unsafe {
            let reference = &*self.buffer.get();
            &*(reference.as_ptr() as *const PacketHeader)
        }
    }

    #[inline]
    pub fn read_shallow<TO>(&mut self) -> TO {
        self.align::<TO>();
        let size = mem::size_of::<TO>();

        let mut object = MaybeUninit::<TO>::uninit();
        unsafe {
            ptr::copy_nonoverlapping(
                self.buffer.get_mut()[self.read..].as_ptr(),
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
            ptr::copy_nonoverlapping(
                self.buffer.get_mut()[self.read..].as_ptr(),
                ptr as *mut u8,
                size,
            );
        }
        self.read += size;
    }

    #[inline]
    pub fn read_shallow_under_nullable_ptr<TO>(&mut self) -> *const TO {
        if self.read_shallow::<u8>() != 1 {
            ptr::null()
        } else {
            self.align::<TO>();
            let ptr = unsafe { self.buffer.get_mut().as_ptr().add(self.read) } as *const TO;
            self.read += mem::size_of::<TO>();
            ptr
        }
    }

    #[inline]
    pub fn read_mut_shallow_under_nullable_ptr<TO>(&mut self) -> *mut TO {
        self.read_shallow_under_nullable_ptr::<TO>() as *mut TO
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to destination.
    #[inline]
    pub unsafe fn read_mut_shallow_under_nullable_ptr_at<TO>(&mut self, dst: *mut TO) {
        let src = self.read_mut_shallow_under_nullable_ptr();
        ptr::copy_nonoverlapping(src, dst, 1);
    }

    #[inline]
    pub fn read_null_str(&mut self) -> *const c_char {
        if self.buffer.get_mut()[self.read] == 0 {
            self.read += 1;
            return ptr::null();
        }

        let start = self.read;
        while self.buffer.get_mut()[self.read] != 0 {
            self.read += 1;
        }

        self.read += 1;
        self.buffer.get_mut()[start..self.read].as_ptr() as *const c_char
    }

    #[inline]
    pub fn read_is_null_ptr(&mut self) -> bool {
        let is_null = self.buffer.get_mut()[self.read] == 1;
        self.read += 1;
        is_null
    }

    #[inline]
    pub fn read_and_allocate_vk_array_count<TA>(&mut self) -> (u32, *mut TA) {
        let count = self.read_shallow::<u32>();
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
        let c = self.read_shallow::<u32>();
        *count = c;
        if !destination.is_null() {
            ptr::copy_nonoverlapping(
                self.buffer.get_mut()[self.read..].as_ptr() as *const TO,
                destination,
                c as usize,
            );
            self.read += c as usize * mem::size_of::<TO>();
        }
    }

    #[inline]
    pub fn read_deep<TO>(&mut self) -> *const TO
    where
        TO: CDeserialize<Packet<'c, T>>,
    {
        match self.read_shallow::<u8>() {
            1 => unsafe { TO::deserialize_ref(self) },
            _ => ptr::null(),
        }
    }

    #[inline]
    pub fn read_mut_deep<TO>(&mut self) -> *mut TO
    where
        TO: CDeserialize<Packet<'c, T>>,
    {
        self.read_deep::<TO>() as *mut TO
    }

    /// # Safety
    /// Caller must ensure to pass a valid pointer to destination.
    #[inline]
    pub unsafe fn read_mut_deep_at<TO>(&mut self, dst: *mut TO)
    where
        TO: CDeserialize<Packet<'c, T>>,
    {
        if self.read_shallow::<u8>() == 1 {
            TO::deserialize_to(self, dst);
        }
    }

    #[inline]
    pub fn read_deep_double<TO>(&mut self) -> *const *const TO
    where
        TO: CDeserialize<Packet<'c, T>>,
    {
        unimplemented!()
    }

    pub fn write_response(mut self, destination: Option<u64>) -> PacketWriter<'c, T> {
        if self.buffer.get_mut().len() != self.read {
            panic!("Packet buffer is not fully read.");
        }

        let destination = match destination {
            Some(d) => Destination::Handler(d),
            None => match self.header().sender_thread_id {
                Some(thread_id) => Destination::Thread(thread_id),
                None => panic!("packet does not have sender thread id or destination is not set"),
            },
        };

        let read_buffer =
            mem::replace(&mut self.buffer, UnsafeCell::new(AVec::with_capacity(0, 0))).into_inner();
        PacketWriter::new(
            self.connection,
            self.connection.pop_buffer(),
            read_buffer,
            destination,
        )
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
        if self.buffer.get_mut().capacity() != 0 {
            if self.buffer.get_mut().len() != self.read {
                panic!("Packet buffer is not fully read.");
            }

            let buffer = mem::replace(&mut self.buffer, UnsafeCell::new(AVec::with_capacity(0, 0)))
                .into_inner();
            self.connection.push_buffer(buffer);
        }
    }
}

unsafe impl<T> CDumpReader for Packet<'_, T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    fn align<TO>(&mut self) {
        self.align::<TO>();
    }

    fn add_read(&mut self, len: usize) {
        self.read += len;
    }

    unsafe fn read_raw_slice(&mut self, len: usize) -> *const u8 {
        let s = unsafe { &*self.buffer.get() };
        let ptr = s.as_ptr().add(self.read);
        self.read += len;
        ptr
    }

    unsafe fn as_mut_ptr_at<TO>(&self, index: usize) -> *mut TO {
        let s = &mut *self.buffer.get();
        s.as_mut_ptr().add(index) as *mut TO
    }

    fn get_read(&self) -> usize {
        self.read
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use std::{
        cell::UnsafeCell,
        ffi::CStr,
        mem,
        ptr::{self, NonNull},
    };

    use aligned_vec::{avec, AVec};
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
            avec![0; mem::size_of::<PacketHeader>()],
            AVec::with_capacity(1, 0),
            Destination::Handler(0),
        );
        write(&mut writer);

        let mut packet = Packet::new(
            connection,
            mem::replace(&mut writer.buffer, AVec::with_capacity(0, 0)),
        );
        read(&mut packet);

        assert_eq!(packet.buffer.get_mut().len(), packet.read);

        _ = mem::replace(
            &mut packet.buffer,
            UnsafeCell::new(AVec::with_capacity(0, 0)),
        );
        mem::forget(writer);
        mem::forget(packet);
    }

    #[test]
    fn write_shallow_read() {
        helper(
            |packet| {
                packet.write_shallow(11u8);
                packet.write_shallow(34.13f64);
            },
            |packet| {
                assert_eq!(packet.read_shallow::<u8>(), 11);
                assert_eq!(packet.read_shallow::<f64>(), 34.13);
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
    fn raw_ptr_as_shallow() {
        helper(
            |packet| {
                let obj = 1984f64;
                packet.write_raw_ptr_as_shallow(&obj as *const _);
            },
            |packet| {
                let mut obj = 0f64;
                packet.read_to_raw_ptr(&mut obj as *mut _);
                assert_eq!(1984f64, obj);
            },
        )
    }

    #[test]
    fn shallow_under_nullable_ptr() {
        helper(
            |packet| {
                let obj = 1984f64;
                packet.write_shallow_under_nullable_ptr(&obj as *const f64);
            },
            |packet| {
                let ptr = packet.read_shallow_under_nullable_ptr::<f64>();
                assert_eq!(1984f64, unsafe { *ptr });
            },
        )
    }

    #[test]
    fn shallow_under_nullable_ptr_null_value() {
        helper(
            |packet| {
                packet.write_shallow_under_nullable_ptr(ptr::null::<u32>());
            },
            |packet| {
                let ptr = packet.read_shallow_under_nullable_ptr::<u32>();
                assert_eq!(ptr::null(), ptr);
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

use std::{
    mem,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex, Weak,
    },
    thread::{self, ThreadId},
};

use lockfree::{map::Map, queue::Queue, stack::Stack};
use packet::{Packet, PacketHeader, PacketWriter};
use unsafe_receiver::UnsafeReceiver;
use wie_common::stream::{UnsafeRead, UnsafeWrite};

pub mod packet;
mod unsafe_receiver;

//type StaticStream = wie_transport_vsock::VsockStream;

//static CONNECTION: OnceLock<Connection<StaticStream>> = OnceLock::new();

pub struct Connection<T>
where
    T: UnsafeWrite + UnsafeRead,
{
    stream: T,
    buffer_pool: Stack<Vec<u8>>,
    write_queue: Queue<Vec<u8>>,
    write_mutex: Mutex<()>,
    thread_channels: Map<u64, ThreadChannel>,
}

impl<T> Connection<T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    pub fn new(stream: T) -> Arc<Self> {
        let connection = Arc::new(Self {
            stream,
            buffer_pool: Stack::new(),
            write_queue: Queue::new(),
            write_mutex: Mutex::new(()),
            thread_channels: Map::new(),
        });

        // Create receive thread
        let weak = Arc::downgrade(&connection);
        thread::spawn(move || receive_worker(weak));

        connection
    }

    #[inline]
    pub fn new_packet(&self) -> PacketWriter<T> {
        let buffer = self.pop_buffer();
        PacketWriter::new(self, buffer)
    }

    pub(crate) fn send(&self, buffer: Vec<u8>) {
        self.push_to_write_queue(buffer, None);
        self.notify_write_thread();
    }

    pub(crate) fn send_with_response(&self, buffer: Vec<u8>) -> Packet<'_, T> {
        let thread_id = thread::current().id();
        self.push_to_write_queue(buffer, Some(thread_id));

        if let Ok(guard) = self.write_mutex.try_lock() {
            self.write_impl();

            // Drop here for faster release.
            mem::drop(guard);

            self.notify_write_thread();
        }

        // Get channel
        let thread_id_raw: u64 = unsafe { mem::transmute(thread_id) };
        let channel;
        loop {
            match self.thread_channels.get(&thread_id_raw) {
                Some(a) => {
                    channel = a;
                    break;
                }
                None => {
                    let (sender, receiver) = mpsc::channel();
                    _ = self.thread_channels.insert(
                        thread_id_raw,
                        ThreadChannel {
                            sender,
                            receiver: UnsafeReceiver(receiver),
                        },
                    )
                }
            }
        }

        // Wait for packet
        let buffer = channel
            .1
            .receiver
            .recv()
            .expect("expected data from channel");
        Packet::new(self, buffer)
    }

    #[inline]
    pub(crate) fn push_buffer(&self, mut buffer: Vec<u8>) {
        unsafe { buffer.set_len(mem::size_of::<PacketHeader>()) }
        self.buffer_pool.push(buffer);
    }

    #[inline]
    fn pop_buffer(&self) -> Vec<u8> {
        match self.buffer_pool.pop() {
            Some(buffer) => buffer,
            None => vec![0; mem::size_of::<PacketHeader>()],
        }
    }

    #[inline]
    fn notify_write_thread(&self) {
        todo!()
    }

    fn write_impl(&self) {
        while let Some(buffer) = self.write_queue.pop() {
            // Safety: Synchronized via self.write_mutex
            unsafe { self.stream.write_unsafe(&buffer) }.unwrap();
        }
    }

    #[inline]
    fn push_to_write_queue(&self, buffer: Vec<u8>, thread_id: Option<ThreadId>) {
        self.push_to_write_queue_with_header(self.make_packet_header(&buffer, thread_id), buffer);
    }

    #[inline]
    fn make_packet_header(&self, buffer: &[u8], thread_id: Option<ThreadId>) -> PacketHeader {
        PacketHeader {
            length: buffer.len(),
            thread_id,
        }
    }

    #[inline]
    fn push_to_write_queue_with_header(&self, header: PacketHeader, mut buffer: Vec<u8>) {
        Self::set_header(header, &mut buffer);
        self.write_queue.push(buffer);
    }

    #[inline]
    fn set_header(header: PacketHeader, buffer: &mut Vec<u8>) {
        let src = &header as *const _ as *const u8;
        let dst = buffer.as_mut_ptr();
        unsafe { std::ptr::copy_nonoverlapping(src, dst, mem::size_of::<PacketHeader>()) };
    }
}

struct ThreadChannel {
    sender: Sender<Vec<u8>>,
    // Safety: Field access must be externally synchronized.
    receiver: UnsafeReceiver<Vec<u8>>,
}

fn receive_worker<T>(weak: Weak<Connection<T>>)
where
    T: UnsafeRead + UnsafeWrite,
{
    while let Some(connection) = weak.upgrade() {
        for _ in 0..64 {}
    }
}

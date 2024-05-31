use std::{
    collections::HashMap,
    mem,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex, Weak,
    },
    thread::{self, ThreadId},
};

use lockfree::{map::Map, queue::Queue, stack::Stack};
use packet::{Destination, Packet, PacketHeader, PacketWriter};
use rsevents::{AutoResetEvent, Awaitable};
use unsafe_receiver::UnsafeReceiver;
use wie_common::stream::{UnsafeRead, UnsafeWrite};

pub mod packet;
mod unsafe_receiver;

const DEFAULT_PART_SIZE: usize = 4096;

//type StaticStream = wie_transport_vsock::VsockStream;

//static CONNECTION: OnceLock<Connection<StaticStream>> = OnceLock::new();

pub struct Connection<T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    stream: T,
    buffer_pool: Stack<Vec<u8>>,
    write_queue: Queue<Vec<u8>>,
    write_mutex: Mutex<()>,
    thread_channels: Map<u64, ThreadChannel>,
    write_reset_event: AutoResetEvent,
    handlers: HashMap<u64, Handler<T>>,
}

type Handler<T> = Box<dyn Fn(Packet<T>) + Send + Sync>;

impl<T> Connection<T>
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    pub fn new(
        stream: T,
        handlers: HashMap<u64, Handler<T>>,
        part_size: Option<usize>,
    ) -> Arc<Self> {
        let part_size = part_size.unwrap_or(DEFAULT_PART_SIZE);

        let connection = Arc::new(Self {
            stream,
            buffer_pool: Stack::new(),
            write_queue: Queue::new(),
            write_mutex: Mutex::new(()),
            thread_channels: Map::new(),
            write_reset_event: AutoResetEvent::new(rsevents::EventState::Unset),
            handlers,
        });

        // Create write thread
        let weak = Arc::downgrade(&connection);
        thread::spawn(move || write_worker(weak));

        // Create receive thread
        let weak = Arc::downgrade(&connection);
        thread::spawn(move || receive_worker(weak, part_size));

        connection
    }

    #[inline]
    pub fn new_packet(&self, destination: u64) -> PacketWriter<T> {
        let buffer = self.pop_buffer();
        PacketWriter::new(self, buffer, Destination::Handler(destination))
    }

    pub(crate) fn send(&self, mut buffer: Vec<u8>) {
        profiling::scope!("send packet");

        Self::update_header(&mut buffer, None);
        self.write_queue.push(buffer);
        self.notify_write_thread();
    }

    pub(crate) fn send_with_response(&self, mut buffer: Vec<u8>) -> Packet<'_, T> {
        profiling::scope!("send packet");

        let thread_id = thread::current().id();
        Self::update_header(&mut buffer, Some(thread_id));

        if let Ok(_guard) = self.write_mutex.try_lock() {
            profiling::scope!("self write");
            self.write_impl(&buffer);
        } else {
            self.write_queue.push(buffer.clone());
        }
        self.notify_write_thread();

        profiling::scope!("wait for response");

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
        self.write_reset_event.set();
    }

    fn write_impl(&self, buffer: &[u8]) {
        unsafe {
            self.stream.write_unsafe(buffer).unwrap();
            self.stream.flush_unsafe().unwrap();
        }
    }

    #[inline]
    fn update_header(buffer: &mut Vec<u8>, sender_thread_id: Option<ThreadId>) {
        let header = unsafe { &mut *(buffer.as_mut_ptr() as *mut PacketHeader) };
        header.length = buffer.len();
        header.sender_thread_id = sender_thread_id;
    }
}

struct ThreadChannel {
    sender: Sender<Vec<u8>>,
    // Safety: Field access must be externally synchronized.
    receiver: UnsafeReceiver<Vec<u8>>,
}

fn write_worker<T>(weak: Weak<Connection<T>>)
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    while let Some(connection) = weak.upgrade() {
        for _ in 0..64 {
            {
                let _guard = connection.write_mutex.lock().unwrap();
                while let Some(buffer) = connection.write_queue.pop() {
                    connection.write_impl(&buffer);
                }
            }

            connection.write_reset_event.wait();
        }
    }
}

fn receive_worker<T>(weak: Weak<Connection<T>>, part_size: usize)
where
    T: UnsafeWrite + UnsafeRead + Send + Sync + 'static,
{
    const MIN_PACKET_SIZE: usize = mem::size_of::<PacketHeader>();

    let mut buffer = vec![0u8; part_size];
    let mut offset = 0;

    let mut packet = Vec::new();
    let mut packet_length = MIN_PACKET_SIZE;
    let mut set_packet_length = false;

    while let Some(connection) = weak.upgrade() {
        for _ in 0..64 {
            // Safety: Only one thread is reading from the stream.
            let read = match unsafe { connection.stream.read_unsafe(&mut buffer) } {
                Ok(read) => read,
                Err(_) => panic!("failed to read from stream"),
            };

            // Read packet length
            if !set_packet_length && read >= mem::size_of::<usize>() {
                packet_length = unsafe { *(buffer.as_ptr() as *const usize) };
                set_packet_length = true;
            }

            while offset != read {
                if packet.is_empty() {
                    profiling::scope!("reading packet");
                }

                let r = (packet_length - packet.len()).min(read);
                packet.extend_from_slice(&buffer[offset..r]);
                offset += r;

                if packet.len() < mem::size_of::<usize>() {
                    break;
                } else if !set_packet_length {
                    packet_length = unsafe { *(packet.as_ptr() as *const usize) };
                    set_packet_length = true;
                }

                if packet.len() == packet_length {
                    let header = unsafe { &*(packet.as_ptr() as *const PacketHeader) };
                    match header.destination {
                        Destination::Thread(thread_id) => {
                            let thread_id_raw = unsafe { mem::transmute(thread_id) };
                            let channel = connection.thread_channels.get(&thread_id_raw).unwrap();
                            channel.1.sender.send(packet).unwrap();
                        }
                        Destination::Handler(handler_id) => {
                            let connection = connection.clone();
                            rayon::spawn(move || {
                                profiling::scope!("handling packet");

                                let handler = connection.handlers.get(&handler_id).unwrap();
                                handler(Packet::new(&connection, packet));
                            });
                        }
                    }

                    packet = connection.pop_buffer();
                    packet.clear();
                    packet_length = MIN_PACKET_SIZE;
                    set_packet_length = false;

                    profiling::finish_frame!();
                }
            }

            offset = 0;
        }
    }

    tracing::info!("receive worker finished");
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use crate::{packet::Packet, Connection, Handler};
    use rsevents::{AutoResetEvent, Awaitable};
    use rstest::rstest;
    use std::{
        collections::HashMap,
        net::{TcpListener, TcpStream},
        sync::Arc,
    };
    use wie_common::stream::mock::MockStream;

    fn new_mock_connection(
        part_size: Option<usize>,
        server_handlers: HashMap<u64, Handler<MockStream>>,
        client_handlers: HashMap<u64, Handler<MockStream>>,
    ) -> (Arc<Connection<MockStream>>, Arc<Connection<MockStream>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();

        (
            Connection::new(server.into(), server_handlers, part_size),
            Connection::new(client.into(), client_handlers, part_size),
        )
    }

    #[rstest]
    #[case(None)]
    #[case(Some(3))]
    #[case(Some(15))]
    fn send(#[case] part_size: Option<usize>) {
        static SERVER_RESET_EVENT: AutoResetEvent =
            AutoResetEvent::new(rsevents::EventState::Unset);
        static CLIENT_RESET_EVENT: AutoResetEvent =
            AutoResetEvent::new(rsevents::EventState::Unset);

        fn client_handle(mut packet: Packet<MockStream>) {
            assert_eq!(2409.04f64, packet.read::<f64>());
            SERVER_RESET_EVENT.set();

            CLIENT_RESET_EVENT.wait();

            let mut packet = packet.write_response(Some(3));
            packet.write(2137u16);
            packet.send();
        }

        fn server_handle(mut packet: Packet<MockStream>) {
            assert_eq!(2137u16, packet.read::<u16>());
            SERVER_RESET_EVENT.set();
        }

        let mut server_handlers: HashMap<u64, Handler<MockStream>> = HashMap::new();
        server_handlers.insert(3, Box::new(server_handle));
        let mut client_handlers: HashMap<u64, Handler<MockStream>> = HashMap::new();
        client_handlers.insert(6, Box::new(client_handle));
        let (server, _client) = new_mock_connection(part_size, server_handlers, client_handlers);

        let mut packet = server.new_packet(6);
        packet.write(2409.04f64);
        packet.send();

        SERVER_RESET_EVENT.wait();
        CLIENT_RESET_EVENT.set();
        SERVER_RESET_EVENT.wait();
    }

    #[rstest]
    #[case(None)]
    #[case(Some(3))]
    #[case(Some(15))]
    fn send_with_response(#[case] part_size: Option<usize>) {
        fn client_handle(mut packet: Packet<MockStream>) {
            assert_eq!(65.420, packet.read::<f64>());
            let mut response = packet.write_response(None);
            response.write(42u32);
            packet = response.send_with_response();

            response = packet.write_response(None);
            response.write(4u128);
            response.send();
        }

        let mut client_handlers: HashMap<u64, Handler<MockStream>> = HashMap::new();
        client_handlers.insert(6, Box::new(client_handle));
        let (server, _client) = new_mock_connection(part_size, HashMap::new(), client_handlers);

        let mut packet = server.new_packet(6);
        packet.write(65.420f64);
        let mut response = packet.send_with_response();
        assert_eq!(42u32, response.read::<u32>());

        packet = response.write_response(None);
        response = packet.send_with_response();
        assert_eq!(4u128, response.read::<u128>());
    }
}

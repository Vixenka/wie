use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use super::{UnsafeRead, UnsafeWrite};

pub struct MockStream {
    read: Mutex<TcpStream>,
    write: Mutex<TcpStream>,
}

impl Default for MockStream {
    fn default() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).unwrap();

        Self {
            read: Mutex::new(stream),
            write: Mutex::new(listener.accept().unwrap().0),
        }
    }
}

impl From<TcpStream> for MockStream {
    fn from(inner: TcpStream) -> Self {
        Self {
            read: inner.try_clone().unwrap().into(),
            write: inner.into(),
        }
    }
}

impl Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read.lock().unwrap().read(buf)
    }
}

impl UnsafeRead for MockStream {
    unsafe fn read_unsafe(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read.lock().unwrap().read(buf)
    }
}

impl Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write.lock().unwrap().flush()
    }
}

impl UnsafeWrite for MockStream {
    unsafe fn write_unsafe(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.write.lock().unwrap().write(buf)
    }

    unsafe fn flush_unsafe(&self) -> std::io::Result<()> {
        self.write.lock().unwrap().flush()
    }
}

pub mod errors;

#[cfg(not(target_os = "windows"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

use std::{
    fmt,
    io::{Read, Write},
    num::NonZeroU32,
};

use errors::{VsockConnectionError, VsockCreationError, VsockListenerBindError};

#[cfg(not(target_os = "windows"))]
use unix as imp;
#[cfg(target_os = "windows")]
use windows as imp;

#[derive(Debug)]
pub struct VsockListener {
    socket: Vsock,
}

impl VsockListener {
    pub fn bind(port: u32, max_connections: NonZeroU32) -> Result<Self, VsockListenerBindError> {
        let mut socket = Vsock::new()?;
        imp::bind(&mut socket, port)?;
        imp::listen(&mut socket, max_connections)?;
        Ok(Self { socket })
    }

    pub fn accept(
        &self,
        client_address: Option<VsockAddress>,
    ) -> std::io::Result<(VsockStream, VsockAddress)> {
        let socket = imp::accept(&self.socket, client_address)?;
        Ok((VsockStream { socket: socket.0 }, socket.1))
    }

    /// An iterator over the connections being received on this listener.
    pub fn incoming(&self) -> Incoming {
        Incoming { listener: self }
    }
}

/// An iterator that infinitely accepts connections on a VsockListener.
#[derive(Debug)]
pub struct Incoming<'a> {
    listener: &'a VsockListener,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = std::io::Result<VsockStream>;

    fn next(&mut self) -> Option<std::io::Result<VsockStream>> {
        Some(self.listener.accept(None).map(|p| p.0))
    }
}

#[derive(Debug)]
pub struct VsockStream {
    socket: Vsock,
}

impl VsockStream {
    pub fn connect(address: VsockAddress) -> Result<Self, VsockConnectionError> {
        let mut socket = Vsock::new()?;
        imp::connect(&mut socket, address)?;
        Ok(Self { socket })
    }
}

impl Read for VsockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = imp::recv(&mut self.socket, buf);
        match read >= 0 {
            true => Ok(read as usize),
            false => Err(std::io::Error::last_os_error()),
        }
    }
}

impl Write for VsockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let read = imp::send(&mut self.socket, buf);
        match read >= 0 {
            true => Ok(read as usize),
            false => Err(std::io::Error::last_os_error()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Vsock {
    #[cfg(not(target_os = "windows"))]
    pub(crate) inner: i32,
    #[cfg(target_os = "windows")]
    pub(crate) inner: ::windows::Win32::Networking::WinSock::SOCKET,
}

impl Vsock {
    pub(crate) fn new() -> Result<Self, VsockCreationError> {
        imp::new_socket()
    }
}

impl Drop for Vsock {
    fn drop(&mut self) {
        imp::close(self);
    }
}

#[derive(Debug)]
pub struct VsockAddress {
    pub cid: VsockCid,
    pub port: u32,
}

impl fmt::Display for VsockAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.cid.0, self.port)
    }
}

#[derive(Debug)]
pub struct VsockCid(pub u32);

impl VsockCid {
    pub fn new(cid: u32) -> Self {
        VsockCid(cid)
    }

    pub fn host() -> Self {
        VsockCid(2)
    }
}

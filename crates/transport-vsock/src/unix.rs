use std::{mem, num::NonZeroU32};

use libc::{c_void, sa_family_t, sockaddr, sockaddr_vm};

use crate::{
    errors::{VsockConnectionError, VsockCreationError, VsockListenerBindError},
    Vsock, VsockAddress, VsockCid,
};

pub(crate) fn new_socket() -> Result<Vsock, VsockCreationError> {
    let result = unsafe { libc::socket(libc::AF_VSOCK, libc::SOCK_STREAM, 0) };
    match result != -1 {
        true => Ok(Vsock { inner: result }),
        false => Err(VsockCreationError::SocketCreationFail),
    }
}

pub(crate) fn bind(socket: &mut Vsock, port: u32) -> Result<(), VsockListenerBindError> {
    let address = sockaddr_vm {
        svm_family: libc::AF_VSOCK as sa_family_t,
        svm_reserved1: 0,
        svm_port: port,
        svm_cid: libc::VMADDR_CID_HOST,
        svm_zero: [0u8; 4],
    };

    let result = unsafe {
        libc::bind(
            socket.inner,
            &address as *const sockaddr_vm as *const sockaddr,
            mem::size_of::<sockaddr_vm>() as u32,
        )
    };

    match result >= 0 {
        true => Ok(()),
        false => Err(VsockListenerBindError::Bind(port)),
    }
}

pub(crate) fn listen(
    socket: &mut Vsock,
    max_connections: NonZeroU32,
) -> Result<(), VsockListenerBindError> {
    let result = unsafe { libc::listen(socket.inner, max_connections.get() as i32) };
    match result >= 0 {
        true => Ok(()),
        false => Err(VsockListenerBindError::Listen),
    }
}

pub(crate) fn accept(
    socket: &Vsock,
    client_address: Option<VsockAddress>,
) -> std::io::Result<(Vsock, VsockAddress)> {
    let client_address = client_address.unwrap_or(VsockAddress {
        cid: VsockCid(0),
        port: 0,
    });

    let mut address = sockaddr_vm {
        svm_family: libc::AF_VSOCK as sa_family_t,
        svm_reserved1: 0,
        svm_port: client_address.port,
        svm_cid: client_address.cid.0,
        svm_zero: [0u8; 4],
    };

    let mut vsock_addr_len = mem::size_of::<sockaddr_vm>() as libc::socklen_t;
    let result = unsafe {
        libc::accept(
            socket.inner,
            &mut address as *mut _ as *mut sockaddr,
            &mut vsock_addr_len,
        )
    };

    match result >= 0 {
        true => Ok((
            Vsock { inner: result },
            VsockAddress {
                cid: VsockCid(address.svm_cid),
                port: address.svm_port,
            },
        )),
        false => Err(std::io::Error::last_os_error()),
    }
}

pub(crate) fn connect(
    _socket: &mut Vsock,
    _address: VsockAddress,
) -> Result<(), VsockConnectionError> {
    unimplemented!()
}

pub(crate) fn recv(socket: &mut Vsock, buffer: &mut [u8]) -> isize {
    unsafe {
        libc::recv(
            socket.inner,
            buffer.as_mut_ptr() as *mut _ as *mut c_void,
            buffer.len(),
            0,
        )
    }
}

pub(crate) fn send(socket: &mut Vsock, buffer: &[u8]) -> isize {
    unsafe {
        libc::send(
            socket.inner,
            buffer.as_ptr() as *const _ as *const c_void,
            buffer.len(),
            0,
        )
    }
}

pub(crate) fn close(socket: &mut Vsock) {
    _ = unsafe { libc::close(socket.inner) };
}

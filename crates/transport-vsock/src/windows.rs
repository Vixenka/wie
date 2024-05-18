//! Implementation based on [github.com](https://gist.github.com/tuxxi/85c03d6593d1f121aa439c0a007f1475) - [archive](https://web.archive.org/web/20240518093847/https://gist.github.com/tuxxi/85c03d6593d1f121aa439c0a007f1475)

use std::{ffi::c_void, mem, num::NonZeroU32};

use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{self, ERROR_SUCCESS, GENERIC_READ},
        Networking::WinSock::{
            self, ADDRESS_FAMILY, SEND_RECV_FLAGS, SOCKADDR, SOCK_STREAM, WSADATA,
        },
        Storage::FileSystem::{self, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_EXISTING},
        System::IO,
    },
};

use crate::{
    errors::{
        VsockConnectionError, VsockCreationError, VsockCreationWindowsError, VsockListenerBindError,
    },
    Vsock, VsockAddress, VsockCid,
};

const VIOSOCK_NAME: &str = "\\??\\Viosock";
const IOCTL_GET_AF: u32 = 0x0801300C;

pub(crate) fn new_socket() -> Result<Vsock, VsockCreationError> {
    let mut wsa_data = WSADATA::default();
    let i_res = unsafe { WinSock::WSAStartup(2 << 8 | 2, &mut wsa_data as *mut WSADATA) };

    if i_res != ERROR_SUCCESS.0 as i32 {
        return Err(VsockCreationWindowsError::WSAStartupFail.into());
    }

    let af = viosock_get_af()?;
    let socket = unsafe { WinSock::socket(af.0 as i32, SOCK_STREAM, 0) };

    Ok(Vsock { inner: socket })
}

pub(crate) fn bind(socket: &mut Vsock, port: u32) -> Result<(), VsockListenerBindError> {
    let address = sockaddr_vm {
        svm_family: ADDRESS_FAMILY(40), // AF_VSOCK
        svm_reserved1: 0,               // Unused
        svm_port: port,
        svm_cid: VsockCid::host().0,
    };

    let result = unsafe {
        WinSock::bind(
            socket.inner,
            &address as *const sockaddr_vm as *const SOCKADDR,
            mem::size_of::<sockaddr_vm>() as i32,
        )
    };

    match result >= 0 {
        true => Ok(()),
        false => Err(VsockListenerBindError::Bind(port)),
    }
}

pub(crate) fn listen(
    _socket: &mut Vsock,
    _max_connections: NonZeroU32,
) -> Result<(), VsockListenerBindError> {
    unimplemented!()
}

pub(crate) fn accept(
    _socket: &Vsock,
    _client_address: Option<VsockAddress>,
) -> std::io::Result<(Vsock, VsockAddress)> {
    unimplemented!()
}

pub(crate) fn connect(
    socket: &mut Vsock,
    address: VsockAddress,
) -> Result<(), VsockConnectionError> {
    let address2 = sockaddr_vm {
        svm_family: ADDRESS_FAMILY(40), // AF_VSOCK
        svm_reserved1: 0,               // Unused
        svm_port: address.port,
        svm_cid: address.cid.0,
    };

    let result = unsafe {
        WinSock::connect(
            socket.inner,
            &address2 as *const sockaddr_vm as *const SOCKADDR,
            mem::size_of::<sockaddr_vm>() as i32,
        )
    };

    match result >= 0 {
        true => Ok(()),
        false => Err(VsockConnectionError::Connection(address)),
    }
}

pub(crate) fn recv(socket: &mut Vsock, buffer: &mut [u8]) -> isize {
    (unsafe { WinSock::recv(socket.inner, buffer, SEND_RECV_FLAGS(0)) }) as isize
}

pub(crate) fn send(socket: &mut Vsock, buffer: &[u8]) -> isize {
    (unsafe { WinSock::send(socket.inner, buffer, SEND_RECV_FLAGS(0)) }) as isize
}

pub(crate) fn close(socket: &mut Vsock) {
    _ = unsafe { WinSock::closesocket(socket.inner) };
}

/// https://github.com/virtio-win/kvm-guest-drivers-windows/blob/657ad7efb539dd186cec1fd33cf31ba710f5dfb1/viosock/inc/vio_sockets.h#L73
fn viosock_get_af() -> Result<ADDRESS_FAMILY, windows::core::Error> {
    let h_device = unsafe {
        FileSystem::CreateFileA(
            PCSTR::from_raw(VIOSOCK_NAME.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }?;

    let mut af = ADDRESS_FAMILY::default();
    unsafe {
        IO::DeviceIoControl(
            h_device,
            IOCTL_GET_AF,
            None,
            0,
            Some(&mut af as *mut ADDRESS_FAMILY as *mut c_void),
            mem::size_of::<ADDRESS_FAMILY>().try_into().unwrap(),
            None,
            None,
        )
    }?;

    // SAFETY: h_device is created before by WinAPI, and checked by Result then h_device must be valid.
    unsafe { Foundation::CloseHandle(h_device) }?;

    Ok(af)
}

#[allow(non_camel_case_types)]
#[repr(C)]
struct sockaddr_vm {
    svm_family: ADDRESS_FAMILY, /* Address family: AF_VSOCK */
    svm_reserved1: u16,
    svm_port: u32, /* Port # in host byte order */
    svm_cid: u32,  /* Address in host byte order */
}

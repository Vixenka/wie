use thiserror::Error;
use wie_common::errors::WindowsError;

use crate::VsockAddress;

#[derive(Error, Debug)]
pub enum VsockCreationError {
    #[error("unable to create a new socket")]
    SocketCreationFail,
    #[error("{0}")]
    Windows(#[from] VsockCreationWindowsError),
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for VsockCreationError {
    fn from(value: windows::core::Error) -> Self {
        Self::Windows(VsockCreationWindowsError::BuiltIn(value.into()))
    }
}

#[derive(Error, Debug)]
pub enum VsockCreationWindowsError {
    #[error("WSAStartup failed")]
    WSAStartupFail,
    #[error("{0}")]
    BuiltIn(#[from] WindowsError),
}

#[derive(Error, Debug)]
pub enum VsockListenerBindError {
    #[error("{0}")]
    Creation(#[from] VsockCreationError),
    #[error("unable bind to port {0}")]
    Bind(u32),
}

#[derive(Error, Debug)]
pub enum VsockConnectionError {
    #[error("{0}")]
    Creation(#[from] VsockCreationError),
    #[error("unable connect to {0}")]
    Connection(VsockAddress),
}

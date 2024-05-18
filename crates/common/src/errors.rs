use std::fmt;

use thiserror::Error;

#[derive(Error, Debug)]
pub struct WindowsError {
    code: i32,
    message: String,
}

impl fmt::Display for WindowsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for WindowsError {
    fn from(value: windows::core::Error) -> Self {
        WindowsError {
            code: unsafe { std::mem::transmute(value.code()) },
            message: value.message(),
        }
    }
}

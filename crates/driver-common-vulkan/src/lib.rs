use std::{ffi::c_void, ptr};

use cdump::{CDumpReader, CDumpWriter};

pub mod generated;

#[doc = "https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_DEFINE_NON_DISPATCHABLE_HANDLE.html"]
pub type NonDisposableHandle = u64;

/// # Safety
/// Ptr must be a valid pointer to a T or null.
pub unsafe fn to_reference<T>(ptr: *const T) -> Option<&'static T> {
    match ptr.is_null() {
        true => None,
        false => unsafe { Some(&*ptr) },
    }
}

pub(crate) unsafe fn p_next_serializer<T: CDumpWriter>(_buf: &mut T, _obj: *const c_void) {}

pub(crate) unsafe fn p_next_deserializer<T: CDumpReader>(_buf: &mut T) -> *mut c_void {
    ptr::null_mut()
}

pub(crate) unsafe fn unimplemented_serializer<T: CDumpWriter>(_buf: &mut T, _obj: *const c_void) {
    unimplemented!("unimplemented_serializer");
}

pub(crate) unsafe fn unimplemented_deserializer<T: CDumpReader>(_buf: &mut T) -> *const c_void {
    unimplemented!("unimplemented_deserializer");
}

pub(crate) unsafe fn unimplemented_deserializer_mut<T: CDumpReader>(_buf: &mut T) -> *mut c_void {
    unimplemented!("unimplemented_deserializer_mut");
}

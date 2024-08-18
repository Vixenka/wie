use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
};

use cdump::{CDumpReader, CDumpWriter};

pub mod generated;

#[doc = "https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_DEFINE_NON_DISPATCHABLE_HANDLE.html"]
pub type NonDisposableHandle = u64;

/// # Safety
/// Ptr must be a null or valid pointer to C array of T with len elements.
pub unsafe fn unpack_vk_array<T>(ptr: *const T, len: usize) -> Option<&'static [T]> {
    (!ptr.is_null() && len != 0).then(|| std::slice::from_raw_parts(ptr, len))
}

/// # Safety
/// Ptr must be a null or valid pointer to C string, ended with null byte.
pub unsafe fn unpack_cstr(ptr: *const c_char) -> Option<&'static str> {
    (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_str().unwrap())
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

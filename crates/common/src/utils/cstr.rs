use std::{
    ffi::{c_char, CStr},
    ptr,
};

/// # Safety
/// Other must be a valid C string, terminated with a null byte.
pub unsafe fn eq_inline(s: &CStr, other: &[i8]) -> bool {
    let rhs = CStr::from_ptr(other as *const _ as *const i8);
    s == rhs
}

/// # Safety
/// Data must be a valid array of C string, with number of elements equal to len.
pub unsafe fn contains(target: &CStr, data: *const *const c_char, len: usize) -> bool {
    for i in 0..len {
        if CStr::from_ptr(*data.add(i)) == target {
            return true;
        }
    }
    false
}

/// # Safety
/// Data must be a valid array of C string, with number of elements equal to len.
/// Returned type and value parameter must outlive the data parameter.
pub unsafe fn extend_array(
    value: &CStr,
    data: &mut *const *const c_char,
    len: &mut u32,
) -> Vec<*const c_char> {
    let buf = if *len == 0 {
        vec![value.as_ptr()]
    } else {
        let len_usize = *len as usize;
        let mut buf: Vec<*const c_char> = Vec::with_capacity(len_usize + 1);
        ptr::copy_nonoverlapping(*data, buf.as_mut_ptr(), len_usize);
        buf.set_len(len_usize);
        buf.push(value.as_ptr());
        buf
    };

    *data = buf.as_ptr();
    *len += 1;

    buf
}

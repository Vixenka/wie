use std::{
    ffi::{c_char, CStr},
    mem,
};

use ash::vk;

use crate::generated::definitions;

#[no_mangle]
extern "stdcall" fn vk_icdGetInstanceProcAddr(
    _instance: vk::Instance,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name = unsafe { CStr::from_ptr(p_name) };
    let name = name.to_str().expect("UTF-8 valid name");
    let address = definitions::get_function_address(name);

    #[cfg(debug_assertions)]
    if address.is_null() {
        panic!("function `{}` is unsupported in wie-driver-vulkan", name);
    }

    unsafe { mem::transmute(address) }
}

#[no_mangle]
extern "stdcall" fn vk_icdNegotiateLoaderICDInterfaceVersion(
    _p_supported_version: *mut u32,
) -> vk::Result {
    unimplemented!("vk_icdNegotiateLoaderICDInterfaceVersion");
}

#[no_mangle]
extern "stdcall" fn vk_icdGetPhysicalDeviceProcAddr(
    _instance: vk::Instance,
    _p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    unimplemented!("vk_icdGetPhysicalDeviceProcAddr");
}

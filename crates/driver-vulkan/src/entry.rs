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
extern "stdcall" fn vk_icdGetPhysicalDeviceProcAddr(
    _instance: vk::Instance,
    _p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    println!("bao2");
    std::fs::write(
        "C:\\Users\\Vixen\\Desktop\\wie-logs\\vk_icdGetPhysicalDeviceProcAddr.txt",
        "whoa",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    unimplemented!();
}

#[no_mangle]
extern "stdcall" fn vk_icdNegotiateLoaderICDInterfaceVersion(
    _p_supported_version: *mut u32,
) -> vk::Result {
    println!("bao3");
    std::fs::write(
        "C:\\Users\\Vixen\\Desktop\\wie-logs\\vk_icdNegotiateLoaderICDInterfaceVersion.txt",
        "whoa",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    unimplemented!();
}

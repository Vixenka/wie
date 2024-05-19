use std::ffi::c_char;

use ash::vk;

#[no_mangle]
extern "C" fn vk_icdGetInstanceProcAddr(
    _instance: vk::Instance,
    _p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    println!("bao");
    std::fs::write(
        "C:\\Users\\Vixen\\Desktop\\wie-logs\\vk_icdGetInstanceProcAddr.txt",
        "whoa",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    unimplemented!();
}

#[no_mangle]
extern "C" fn vk_icdGetPhysicalDeviceProcAddr(
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
extern "C" fn vk_icdNegotiateLoaderICDInterfaceVersion(
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

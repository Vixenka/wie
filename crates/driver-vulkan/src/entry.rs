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

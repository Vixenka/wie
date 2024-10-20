use std::{
    ffi::{c_char, CStr},
    mem,
};

use ash::vk;
use wie_transport_guest::new_packet;

use crate::generated::definitions;

/// https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderDriverInterface.md
const SUPPORTED_LOADER_ICD_INTERFACE_VERSION: u32 = 7;
static mut CURRENT_LOADER_ICD_INTERFACE_VERSION: u32 = 0;

#[no_mangle]
extern "stdcall" fn vk_icdGetInstanceProcAddr(
    instance: vk::Instance,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    wie_transport_guest::start_connection(crate::transport_handlers::get);

    unsafe {
        if CURRENT_LOADER_ICD_INTERFACE_VERSION == 0 {
            CURRENT_LOADER_ICD_INTERFACE_VERSION = 1;
        }
    }

    let name = unsafe { CStr::from_ptr(p_name) };
    let name = name.to_str().expect("UTF-8 valid name");
    let address = match name {
        "vk_icdGetPhysicalDeviceProcAddr" => vk_icdGetPhysicalDeviceProcAddr as *const _,
        "vk_icdEnumerateAdapterPhysicalDevices" => {
            vk_icdEnumerateAdapterPhysicalDevices as *const _
        }
        _ => definitions::get_function_address(name),
    };

    #[cfg(debug_assertions)]
    if address.is_null() {
        panic!("function `{}` is unsupported in wie-driver-vulkan", name);
    }

    trace!("requested address for function `{}`", name);

    let mut packet = new_packet(1000000000);
    packet.write_shallow(instance);
    unsafe { packet.write_null_str(p_name) };
    let mut response = packet.send_with_response();
    if !response.read_shallow::<bool>() {
        return None;
    }

    unsafe { mem::transmute(address) }
}

#[no_mangle]
extern "stdcall" fn vk_icdNegotiateLoaderICDInterfaceVersion(
    p_supported_version: *mut u32,
) -> vk::Result {
    unsafe {
        if *p_supported_version > SUPPORTED_LOADER_ICD_INTERFACE_VERSION {
            *p_supported_version = SUPPORTED_LOADER_ICD_INTERFACE_VERSION;
        }

        trace!(
            "negotiated loader ICD interface version: {}",
            *p_supported_version
        );

        vk::Result::SUCCESS
    }
}

#[no_mangle]
extern "stdcall" fn vk_icdGetPhysicalDeviceProcAddr(
    _instance: vk::Instance,
    _p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    unimplemented!("vk_icdGetPhysicalDeviceProcAddr");
}

#[repr(C)]
struct Luid {
    data: [u8; vk::LUID_SIZE],
}

/// https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderDriverInterface.md#physical-device-sorting
#[no_mangle]
unsafe extern "stdcall" fn vk_icdEnumerateAdapterPhysicalDevices(
    _instance: vk::Instance,
    _adapter_luid: Luid,
    _p_physical_device_count: *mut u32,
    _p_physical_devices: *mut vk::PhysicalDevice,
) -> vk::Result {
    unimplemented!("vk_icdEnumerateAdapterPhysicalDevices");
}

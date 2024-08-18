use std::ffi::{c_char, CStr};

use wie_common::utils::{cstr, env};
use wie_driver_common_vulkan::{
    generated::vulkan_types::{VkAllocationCallbacks, VkInstanceCreateInfo},
    NonDisposableHandle,
};

use crate::{utils, Packet};

#[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateInstance.html>"]
pub fn vk_create_instance(mut packet: Packet) {
    unsafe fn turn_on_validation_layers(
        p_instance: *mut NonDisposableHandle,
        create_info: *mut VkInstanceCreateInfo,
    ) -> Option<Vec<*const c_char>> {
        const LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

        let create_info = &mut *create_info;
        if cstr::contains(
            LAYER_NAME,
            create_info.pp_enabled_layer_names,
            create_info.enabled_layer_count as usize,
        ) {
            info!("Vulkan's validation layers are already enabled.");
            return None;
        }

        let layers = utils::instance::get_layer_properties(*p_instance);
        for layer in layers {
            if !cstr::eq_inline(LAYER_NAME, &layer.layer_name) {
                continue;
            }

            info!("Enabled Vulkan's validation layers.");
            return Some(cstr::extend_array(
                LAYER_NAME,
                &mut create_info.pp_enabled_layer_names,
                &mut create_info.enabled_layer_count,
            ));
        }

        error!(
            r#"Vulkan's validation layers are not available on host.

Make sure to have it properly installed in system.
- Linux Arch: https://wiki.archlinux.org/title/Vulkan#Installation"#
        );
        None
    }

    let p_create_info: *mut VkInstanceCreateInfo = packet.read_mut_deep();
    let p_allocator: *const VkAllocationCallbacks = packet.read_deep();
    let p_instance: *mut NonDisposableHandle = packet.read_mut_shallow_under_nullable_ptr();
    unsafe {
        trace!(
            "called vkCreateInstance({:?}, {:?}, {:?})",
            p_create_info.as_ref(),
            p_allocator.as_ref(),
            p_instance.as_ref()
        );
    }

    let _outlive_buf =
        match crate::ENABLE_VALIDATION_LAYERS || env::is_active("VK_VALIDATION_LAYERS") {
            true => unsafe { turn_on_validation_layers(p_instance, p_create_info) },
            false => None,
        };

    unsafe {
        trace!("updated data {:?}", p_create_info.as_ref(),);
    }

    let result = unsafe {
        (crate::FUNCTION_ADDRESS_TABLE.vk_create_instance)(p_create_info, p_allocator, p_instance)
    };

    let mut response = packet.write_response(None);
    response.write_shallow_under_nullable_ptr(p_instance);
    response.write_shallow(result);
    response.send();
}

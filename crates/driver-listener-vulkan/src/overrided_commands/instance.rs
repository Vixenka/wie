use std::{
    ffi::{c_char, c_void, CStr},
    mem, ptr,
};

use ash::vk;
use wie_common::utils::{cstr, env};
use wie_driver_common_vulkan::{
    generated::vulkan_types::{
        VkAllocationCallbacks, VkDebugUtilsMessengerCreateInfoEXT, VkInstanceCreateInfo,
    },
    NonDisposableHandle,
};

use crate::{entry, utils};

const VALIDATION_LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

#[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateInstance.html>"]
pub unsafe fn vk_create_instance(
    p_create_info: *const VkInstanceCreateInfo,
    p_allocator: *const VkAllocationCallbacks,
    p_instance: *mut NonDisposableHandle,
) -> u32 {
    let p_create_info = p_create_info as *mut _;
    let ((_outlive_buf, layers), (_outlive_buf2, extensions)) =
        match crate::ENABLE_VALIDATION_LAYERS || env::is_active("VK_VALIDATION_LAYERS") {
            true => (
                turn_on_validation_layers(p_create_info),
                turn_on_debug_utils_extension(p_create_info),
            ),
            false => ((None, false), (None, false)),
        };

    trace!("updated data {:?}", p_create_info.as_ref());
    let result =
        (crate::FUNCTION_ADDRESS_TABLE.vk_create_instance)(p_create_info, p_allocator, p_instance);

    if layers && extensions {
        create_log_callback(*p_instance);
    }

    result
}

unsafe fn turn_on_validation_layers(
    create_info: *mut VkInstanceCreateInfo,
) -> (Option<Vec<*const c_char>>, bool) {
    let create_info = &mut *create_info;
    if cstr::contains(
        VALIDATION_LAYER_NAME,
        create_info.pp_enabled_layer_names,
        create_info.enabled_layer_count as usize,
    ) {
        info!("Vulkan's validation layers are already enabled.");
        return (None, true);
    }

    let layers = utils::instance::get_layer_properties();
    for layer in layers {
        if !cstr::eq_inline(VALIDATION_LAYER_NAME, &layer.layer_name) {
            continue;
        }

        info!("Enabled Vulkan's validation layers.");
        return (
            Some(cstr::extend_array(
                VALIDATION_LAYER_NAME,
                &mut create_info.pp_enabled_layer_names,
                &mut create_info.enabled_layer_count,
            )),
            true,
        );
    }

    error!(
        r#"Vulkan's validation layers are not available on host.

Make sure to have it properly installed in system.
- Linux Arch: https://wiki.archlinux.org/title/Vulkan#Installation"#
    );
    (None, false)
}

unsafe fn turn_on_debug_utils_extension(
    create_info: *mut VkInstanceCreateInfo,
) -> (Option<Vec<*const c_char>>, bool) {
    const EXTENSION_NAME: &CStr = c"VK_EXT_debug_utils";

    let create_info = &mut *create_info;
    if cstr::contains(
        EXTENSION_NAME,
        create_info.pp_enabled_extension_names,
        create_info.enabled_extension_count as usize,
    ) {
        info!("Vulkan's debug utils extension already enabled.");
        return (None, true);
    }

    let extensions = utils::instance::get_extension_properties(ptr::null());
    for extension in extensions {
        if !cstr::eq_inline(EXTENSION_NAME, &extension.extension_name) {
            continue;
        }

        info!("Enabled Vulkan's debug utils extension.");
        return (
            Some(cstr::extend_array(
                EXTENSION_NAME,
                &mut create_info.pp_enabled_extension_names,
                &mut create_info.enabled_extension_count,
            )),
            true,
        );
    }

    error!("Vulkan's debug utils extension is not available on host.");
    (None, false)
}

unsafe fn create_log_callback(instance: NonDisposableHandle) {
    if !entry::make_sure_function_is_loaded(instance, c"vkCreateDebugUtilsMessengerEXT") {
        return;
    }

    let create_info = mem::transmute::<
        vk::DebugUtilsMessengerCreateInfoEXT,
        VkDebugUtilsMessengerCreateInfoEXT,
    >(vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        pfn_user_callback: Some(log_callback),
        p_user_data: ptr::null_mut(),
        ..Default::default()
    });

    let mut p_messenger = 0;
    let result = (crate::FUNCTION_ADDRESS_TABLE.vk_create_debug_utils_messenger_ext)(
        instance,
        &create_info,
        ptr::null(),
        &mut p_messenger,
    );

    if result != vk::Result::SUCCESS.as_raw() as u32 {
        error!("Failed to create debug utils messenger: {:?}", result);
    } else {
        info!("Debug utils log callback created.");
    }
}

unsafe extern "system" fn log_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    super::debug::process_log(message_severity, message_type, p_callback_data);
    vk::FALSE
}

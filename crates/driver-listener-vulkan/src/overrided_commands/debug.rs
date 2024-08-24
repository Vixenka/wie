use std::ffi::{c_char, c_void, CStr};

use ash::vk;
use log::Level;
use wie_driver_common_vulkan::{
    generated::vulkan_types::{
        VkAllocationCallbacks, VkDebugReportCallbackCreateInfoEXT,
        VkDebugUtilsMessengerCreateInfoEXT,
    },
    NonDisposableHandle,
};

pub unsafe fn process_log(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
) {
    let level = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => Level::Warn,
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => Level::Error,
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => Level::Info,
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => Level::Debug,
        _ => {
            warn!("Vulkan log severity was not handled.");
            Level::Info
        }
    };

    let prefix = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "Validation",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "Performance",
        _ => {
            warn!("Vulkan log type was not handled.");
            "Unknown"
        }
    };

    let message = match CStr::from_ptr((*p_callback_data).p_message).to_str() {
        Ok(m) => m,
        Err(_) => {
            error!("Vulkan log message throws Utf8Error.");
            return;
        }
    };

    if prefix.is_empty() {
        log!(level, "{}", message);
    } else {
        log!(level, "{}: {}", prefix, message);
    }
}

#[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDebugUtilsMessengerEXT.html>"]
pub unsafe fn vk_create_debug_utils_messenger_ext(
    instance: NonDisposableHandle,
    p_create_info: *const VkDebugUtilsMessengerCreateInfoEXT,
    p_allocator: *const VkAllocationCallbacks,
    p_messenger: *mut NonDisposableHandle,
) -> u32 {
    let p_create_info = p_create_info as *mut VkDebugUtilsMessengerCreateInfoEXT;
    let create_info = &mut *p_create_info;
    if !create_info.p_next.is_null() {
        error!("Field `p_next` is not supported yet for `VkDebugUtilsMessengerCreateInfoEXT`.");
    }

    create_info.message_severity = u32::MAX;
    create_info.message_type = u32::MAX;
    create_info.pfn_user_callback = Some(utils_callback);

    (crate::FUNCTION_ADDRESS_TABLE.vk_create_debug_utils_messenger_ext)(
        instance,
        p_create_info,
        p_allocator,
        p_messenger,
    )
}

#[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDebugReportCallbackEXT.html>"]
pub unsafe fn vk_create_debug_report_callback_ext(
    instance: NonDisposableHandle,
    p_create_info: *const VkDebugReportCallbackCreateInfoEXT,
    p_allocator: *const VkAllocationCallbacks,
    p_callback: *mut NonDisposableHandle,
) -> u32 {
    let p_create_info: *mut VkDebugReportCallbackCreateInfoEXT =
        p_create_info as *mut VkDebugReportCallbackCreateInfoEXT;
    let create_info = &mut *p_create_info;

    create_info.pfn_callback = Some(report_callback);

    (crate::FUNCTION_ADDRESS_TABLE.vk_create_debug_report_callback_ext)(
        instance,
        p_create_info,
        p_allocator,
        p_callback,
    )
}

unsafe extern "system" fn utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    process_log(message_severity, message_type, p_callback_data);

    // TODO: Implement calling the original callback.

    vk::FALSE
}

unsafe extern "system" fn report_callback(
    _flags: vk::DebugReportFlagsEXT,
    _object_type: vk::DebugReportObjectTypeEXT,
    _object: u64,
    _location: usize,
    _message_code: i32,
    _p_layer_prefix: *const c_char,
    _p_message: *const c_char,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    // TODO: Implement calling the original callback.
    vk::FALSE
}

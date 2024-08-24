use std::{ffi::c_char, ptr};

use wie_driver_common_vulkan::generated::vulkan_types::{VkExtensionProperties, VkLayerProperties};

use crate::entry;

pub fn get_layer_properties() -> Vec<VkLayerProperties> {
    unsafe {
        if !entry::make_sure_function_is_loaded(0, c"vkEnumerateInstanceLayerProperties") {
            return Vec::new();
        }

        let mut layer_count = 0;
        (crate::FUNCTION_ADDRESS_TABLE.vk_enumerate_instance_layer_properties)(
            &mut layer_count,
            ptr::null_mut(),
        );

        let mut layers = Vec::with_capacity(layer_count as usize);
        (crate::FUNCTION_ADDRESS_TABLE.vk_enumerate_instance_layer_properties)(
            &mut layer_count,
            layers.as_mut_ptr(),
        );
        layers.set_len(layer_count as usize);
        layers
    }
}

pub fn get_extension_properties(p_layer_name: *const c_char) -> Vec<VkExtensionProperties> {
    unsafe {
        if !entry::make_sure_function_is_loaded(0, c"vkEnumerateInstanceExtensionProperties") {
            return Vec::new();
        }

        let mut extension_count = 0;
        (crate::FUNCTION_ADDRESS_TABLE.vk_enumerate_instance_extension_properties)(
            p_layer_name,
            &mut extension_count,
            ptr::null_mut(),
        );

        let mut extensions = Vec::with_capacity(extension_count as usize);
        (crate::FUNCTION_ADDRESS_TABLE.vk_enumerate_instance_extension_properties)(
            p_layer_name,
            &mut extension_count,
            extensions.as_mut_ptr(),
        );
        extensions.set_len(extension_count as usize);
        extensions
    }
}

use std::ptr;

use wie_driver_common_vulkan::{generated::vulkan_types::VkLayerProperties, NonDisposableHandle};

use crate::entry;

pub fn get_layer_properties(instance: NonDisposableHandle) -> Vec<VkLayerProperties> {
    unsafe {
        if !entry::make_sure_function_is_loaded(instance, c"vkEnumerateInstanceLayerProperties") {
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

use wie_driver_common_vulkan::{
    generated::vulkan_types::{VkAllocationCallbacks, VkDebugUtilsMessengerCreateInfoEXT},
    NonDisposableHandle,
};

use crate::Packet;

#[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateDebugUtilsMessengerEXT.html>"]
fn vk_create_debug_utils_messenger_ext(mut packet: Packet) {
    let instance: NonDisposableHandle = packet.read_shallow();
    let p_create_info: *const VkDebugUtilsMessengerCreateInfoEXT = packet.read_deep();
    let p_allocator: *const VkAllocationCallbacks = packet.read_deep();
    let p_messenger: *mut NonDisposableHandle = packet.read_mut_shallow_under_nullable_ptr();
    unsafe {
        trace!(
            "called vkCreateDebugUtilsMessengerEXT({:?}, {:?}, {:?}, {:?})",
            instance,
            p_create_info.as_ref(),
            p_allocator.as_ref(),
            p_messenger.as_ref()
        );
    }

    let result = unsafe {
        (crate::FUNCTION_ADDRESS_TABLE.vk_create_debug_utils_messenger_ext)(
            instance,
            p_create_info,
            p_allocator,
            p_messenger,
        )
    };

    let mut response = packet.write_response(None);
    unsafe {
        response.write_shallow_under_nullable_ptr(p_messenger);
    }
    response.write_shallow(result);
    response.send();
}

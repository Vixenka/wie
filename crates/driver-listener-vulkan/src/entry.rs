use std::ffi::CStr;

use ash::vk::{self, Handle};
use wie_driver_common_vulkan::NonDisposableHandle;

use crate::{HandlerMap, Packet};

pub fn make_sure_function_is_loaded(instance: NonDisposableHandle, name: &CStr) -> bool {
    let str_name = name.to_str().expect("UTF-8 valid name");
    if !request_address_for_function(instance, name, str_name) {
        error!("Failed to request address for {str_name} on host.");
        return false;
    }
    true
}

pub fn request_address_for_function(
    instance: NonDisposableHandle,
    c_name: &CStr,
    str_name: &str,
) -> bool {
    // NOTE: For different instance driver can return different addresses, but for now we just use the same address for
    //       all instances. This can be a problem in the future.
    let address = unsafe {
        crate::get_or_init_entry()
            .get_instance_proc_addr(vk::Instance::from_raw(instance), c_name.as_ptr())
    };
    if address.is_some() {
        unsafe { crate::FUNCTION_ADDRESS_TABLE.set_address(str_name, address) }
        return true;
    }
    false
}

pub fn register_handlers_to(map: &mut HandlerMap) {
    map.insert(1000000000, Box::new(vk_icd_get_instance_proc_addr));
}

fn vk_icd_get_instance_proc_addr(mut packet: Packet) {
    let instance = packet.read_shallow::<vk::Instance>();
    let p_name = packet.read_null_str();
    let c_name = unsafe { CStr::from_ptr(p_name) };

    let str_name = c_name.to_str().expect("UTF-8 valid name");
    trace!("requested address for function `{str_name}`");

    let address = request_address_for_function(instance.as_raw(), c_name, str_name);

    let mut response = packet.write_response(None);
    response.write_shallow(address);
    response.send();
}

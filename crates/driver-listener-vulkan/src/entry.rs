use std::ffi::CStr;

use crate::{HandlerMap, Packet};

pub fn register_handlers_to(map: &mut HandlerMap) {
    map.insert(1000000000, Box::new(vk_icd_get_instance_proc_addr));
}

fn vk_icd_get_instance_proc_addr(mut packet: Packet) {
    let p_name = packet.read_null_str();
    let name = unsafe { CStr::from_ptr(p_name) }
        .to_str()
        .expect("UTF-8 valid name");

    trace!("requested address for function `{name}`",);

    // TODO: Use Instance parameter to be more accurate.
    let address = unsafe {
        crate::get_or_init_entry().get_instance_proc_addr(ash::vk::Instance::null(), p_name)
    };
    if address.is_some() {
        unsafe { crate::FUNCTION_ADDRESS_TABLE.set_address(name, address) }
    }

    let mut response = packet.write_response(None);
    response.write_shallow(match address.is_some() {
        true => 1u8,
        false => 0u8,
    });
    response.send();
}

use std::ffi::CStr;

use crate::{HandlerMap, Packet};

pub fn register_handlers_to(map: &mut HandlerMap) {
    map.insert(1000000000, Box::new(vk_icd_get_instance_proc_addr));
}

fn vk_icd_get_instance_proc_addr(mut packet: Packet) {
    trace!(
        "requested address for function `{}`",
        unsafe { CStr::from_ptr(packet.read_null_str()) }
            .to_str()
            .expect("UTF-8 valid name")
    );

    let mut response = packet.write_response(None);
    response.write(true);
    response.send();
}

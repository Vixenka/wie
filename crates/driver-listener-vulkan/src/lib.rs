use std::collections::HashMap;

use wie_transport::Handler;
use wie_transport_vsock::VsockStream;

#[macro_use]
extern crate log;

pub(crate) mod generated;

type HandlerMap = HashMap<u64, Handler<VsockStream>>;
type Packet<'c> = wie_transport::packet::Packet<'c, VsockStream>;

pub fn register_handlers_to(map: &mut HandlerMap) {
    generated::handlers::register_handlers_to(map);
}

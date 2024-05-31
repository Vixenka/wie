use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use wie_transport::{packet::PacketWriter, Connection};
use wie_transport_vsock::{VsockAddress, VsockCid, VsockStream};

pub type Handler = wie_transport::Handler<VsockStream>;

static CONNECTION: OnceLock<Arc<Connection<VsockStream>>> = OnceLock::new();

pub fn start_connection<T>(handlers: T)
where
    T: FnOnce() -> HashMap<u64, Handler>,
{
    if CONNECTION.get().is_some() {
        return;
    }

    let stream = VsockStream::connect(VsockAddress {
        cid: VsockCid::host(),
        port: 13001,
    })
    .unwrap();

    CONNECTION
        .set(Connection::new(stream, handlers(), None))
        .unwrap()
}

#[inline]
pub fn get_connection() -> &'static Arc<Connection<VsockStream>> {
    CONNECTION.get().unwrap()
}

#[inline]
pub fn new_packet(destination: u64) -> PacketWriter<'static, VsockStream> {
    get_connection().new_packet(destination)
}

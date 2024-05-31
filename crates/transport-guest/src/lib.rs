use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use wie_transport::{packet::PacketWriter, Connection};
use wie_transport_vsock::{errors::VsockConnectionError, VsockAddress, VsockCid, VsockStream};

pub type Handler = wie_transport::Handler<VsockStream>;

static CONNECTION: OnceLock<Arc<Connection<VsockStream>>> = OnceLock::new();

pub fn start_connection<T>(handlers: T)
where
    T: FnOnce() -> HashMap<u64, Handler>,
{
    if CONNECTION.get().is_some() {
        return;
    }

    tracing_subscriber::fmt::init();
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    let stream = match VsockStream::connect(VsockAddress {
        cid: VsockCid::host(),
        port: 13001,
    }) {
        Ok(stream) => stream,
        Err(e) => match e {
            VsockConnectionError::Creation(e) => panic!("Failed to create vsock connection: {}", e),
            VsockConnectionError::Connection(e) => {
                panic!(
                    "FAILED TO CONNECT TO VSOCK HOST. MAKE SURE THE HOST IS RUNNING. {}",
                    e
                )
            }
        },
    };

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

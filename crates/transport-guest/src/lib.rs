#[macro_use]
extern crate log;

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

    simple_logger::init().unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("{}", panic_info);
        hook(panic_info);
    }));

    let stream = match VsockStream::connect(VsockAddress {
        cid: VsockCid::host(),
        port: 13001,
    }) {
        Ok(stream) => stream,
        Err(e) => match e {
            VsockConnectionError::Creation(e) => panic!("Failed to create vsock connection: {}", e),
            VsockConnectionError::Connection(e) => {
                error!("FAILED TO CONNECT TO VSOCK HOST. MAKE SURE THE HOST LISTENER IS RUNNING.");
                panic!("{}", e)
            }
        },
    };

    info!("Connection established");

    CONNECTION
        .set(Connection::new(stream, handlers(), None))
        .unwrap();
}

#[inline]
pub fn get_connection() -> &'static Arc<Connection<VsockStream>> {
    CONNECTION.get().unwrap()
}

#[inline]
pub fn new_packet(destination: u64) -> PacketWriter<'static, VsockStream> {
    get_connection().new_packet(destination)
}

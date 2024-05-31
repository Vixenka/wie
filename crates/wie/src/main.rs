#[macro_use]
extern crate log;

use std::{collections::HashMap, num::NonZeroU32, thread, time::Duration};

use wie_transport::Connection;
use wie_transport_vsock::VsockListener;

const PORT: u32 = 13001;

fn main() {
    simple_logger::init().unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("{}", panic_info);
        hook(panic_info);
    }));

    info!("Setting up listening socket on port {}", PORT);
    let listener = VsockListener::bind(PORT, NonZeroU32::new(1).unwrap())
        .expect("Failed to set up listening port");

    info!("Waiting for incoming connections...");
    let (stream, _) = listener
        .accept(None)
        .expect("Failed to accept incoming connection");

    info!("Connection established");

    let _what = Connection::new(stream, HashMap::new(), None);
    thread::sleep(Duration::from_secs(60));
}

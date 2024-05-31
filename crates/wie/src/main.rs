use std::{collections::HashMap, num::NonZeroU32};

use wie_transport::Connection;
use wie_transport_vsock::VsockListener;

fn main() {
    tracing_subscriber::fmt::init();
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    tracing::info!("Setting up listening socket on port 9999");
    let listener = VsockListener::bind(9999, NonZeroU32::new(1).unwrap())
        .expect("Failed to set up listening port");

    tracing::info!("Waiting for incoming connections...");
    let (stream, _) = listener
        .accept(None)
        .expect("Failed to accept incoming connection");

    tracing::info!("Connection established");

    Connection::new(stream, HashMap::new(), None);
}

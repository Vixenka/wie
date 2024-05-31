use std::{collections::HashMap, num::NonZeroU32};

use wie_transport::Connection;
use wie_transport_vsock::VsockListener;

const PORT: u32 = 13001;

fn main() {
    tracing_subscriber::fmt::init();
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    tracing::info!("Setting up listening socket on port {}", PORT);
    let listener = VsockListener::bind(PORT, NonZeroU32::new(1).unwrap())
        .expect("Failed to set up listening port");

    tracing::info!("Waiting for incoming connections...");
    let (stream, _) = listener
        .accept(None)
        .expect("Failed to accept incoming connection");

    tracing::info!("Connection established");

    Connection::new(stream, HashMap::new(), None);
}

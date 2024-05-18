use std::{
    io::{Read, Write},
    num::NonZeroU32,
};

use wie_transport_vsock::VsockListener;

fn main() {
    let mut buffer = vec![0; 1024];

    println!("[Host] Setting up listening socket on port 9999");
    let listener = VsockListener::bind(63256, NonZeroU32::new(1).unwrap())
        .expect("Failed to set up listening port");

    let (mut stream, _) = listener.accept(None).unwrap();

    let len = stream
        .read(&mut buffer)
        .expect("Failed to read from stream");
    println!(
        "[Host] Received: {:?}",
        std::str::from_utf8(&buffer[..len]).unwrap()
    );

    let written_bytes = stream
        .write(b"Hello from host")
        .expect("Failed to write to stream");
    if written_bytes == 0 {
        panic!("Failed to write to stream");
    }
}

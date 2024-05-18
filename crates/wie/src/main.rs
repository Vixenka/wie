use std::io::{Read, Write};

use wie_transport_vsock::VsockListener;

fn main() {
    let mut buffer = vec![0; 1024];

    println!("[Host] Setting up listening socket on port 9999");
    let listener = VsockListener::bind(9999).expect("Failed to set up listening port");

    let Ok((mut stream, address)) = listener.accept(None) else {
        println!("[Host] Failed to get vsock_stream");
        return;
    };
    println!(
        "[Host] Accept connection: {:?}, peer addr: {:?}, local addr: {:?}",
        stream, address.cid.0, address.port
    );

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

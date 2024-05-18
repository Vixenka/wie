use std::io::{Read, Write};

use wie_transport_vsock::{VsockAddress, VsockCid, VsockStream};

fn main() {
    let mut buffer = vec![0; 1024];

    let mut stream = VsockStream::connect(VsockAddress {
        cid: VsockCid::host(),
        port: 9999,
    })
    .expect("Connection failed");

    let written_bytes = stream
        .write(b"Hello from guest")
        .expect("Failed to write to stream");
    if written_bytes == 0 {
        panic!("Failed to write to stream");
    }

    let len = stream
        .read(&mut buffer)
        .expect("Failed to read from stream");
    println!(
        "[Guest] Received: {:?}",
        std::str::from_utf8(&buffer[..len]).unwrap()
    );
}

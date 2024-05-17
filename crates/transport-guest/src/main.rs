use std::io::{Read, Write};

use vsock::VsockStream;

const PORT: u32 = 9999;

fn main() {
    let mut buffer = vec![0; 1024];

    let mut stream = VsockStream::connect_with_cid_port(vsock::VMADDR_CID_HOST, PORT)
        .expect("Connection failed");

    let written_bytes = stream
        .write(b"Hello from guest")
        .expect("Failed to write to stream");
    if written_bytes == 0 {
        panic!("Failed to write to stream");
    }
    stream.flush().expect("flush");

    let len = stream
        .read(&mut buffer)
        .expect("Failed to read from stream");
    println!(
        "[Guest] Received: {:?}",
        std::str::from_utf8(&buffer[..len]).unwrap()
    );
}

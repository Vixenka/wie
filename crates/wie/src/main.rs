use std::io::{Read, Write};
use vsock::{VsockAddr, VsockListener, VMADDR_CID_HOST};

const PORT: u32 = 9999;

fn main() {
    let mut buffer = vec![0; 1024];

    println!("[Host] Setting up listening socket on port {PORT}");
    let listener = VsockListener::bind(&VsockAddr::new(VMADDR_CID_HOST, PORT))
        .expect("Failed to set up listening port");

    let Some(Ok(mut stream)) = listener.incoming().next() else {
        println!("[Host] Failed to get vsock_stream");
        return;
    };
    println!(
        "[Host] Accept connection: {:?}, peer addr: {:?}, local addr: {:?}",
        stream,
        stream.peer_addr(),
        stream.local_addr()
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
    stream.flush().expect("flush");
}

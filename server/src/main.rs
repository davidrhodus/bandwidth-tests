use std::io::{Write, Read};
use std::net::{TcpListener, TcpStream};
use std::thread;
use socket2::{Socket, Domain, Type, Protocol};

fn handle_client(mut stream: TcpStream) {
    // Use socket2 to set the buffer size for the TCP socket
    let socket = Socket::from(stream.try_clone().expect("Failed to clone TcpStream"));
    let buffer_size = 1_000_000; // 1 MB buffer size for TCP window
    socket.set_send_buffer_size(buffer_size).expect("Failed to set send buffer size");

    // Create a 1 MB chunk of dummy data to send to the client
    let chunk = vec![0u8; 1_000_000]; // 1 MB of zeroed bytes

    for _ in 0..100 {
        // Send the 1 MB chunk to the client
        if let Err(e) = stream.write_all(&chunk) {
            eprintln!("Failed to send data chunk: {}", e);
            return;
        }
        println!("Sent 1 MB chunk to client");
    }

    println!("Completed 100 chunks transfer to client");
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("Server listening on port 7878...");

    if let Some(stream) = listener.incoming().next() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
                println!("Server exiting after handling one client.");
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

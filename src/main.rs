use bytes::Bytes;
use dns_message_parser::Dns;
use std::error::Error;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::thread;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:5433").unwrap();

    loop {
        let mut buf = [0; 1024];
        let (size, origin) = socket.recv_from(&mut buf).unwrap();
        let socket_clone = socket.try_clone().unwrap();

        thread::spawn(
            move || match handle_request(&buf[..size], origin, &socket_clone) {
                Ok(_) => (),
                Err(e) => println!("{:?}", e),
            },
        );
    }
}

fn handle_request(msg: &[u8], from: SocketAddr, socket: &UdpSocket) -> Result<(), Box<dyn Error>> {
    let bytes = Bytes::copy_from_slice(msg);
    Dns::decode(bytes.clone())?;

    let client = reqwest::blocking::Client::new();

    let mut resp = client
        .post("https://cloudflare-dns.com/dns-query")
        .header("content-type", "application/dns-message")
        .header("accept", "application/dns-message")
        .body(bytes.clone())
        .send()?;

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf)?;
    socket.send_to(&buf, from)?;
    Ok(())
}

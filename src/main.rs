use bytes::Bytes;
use dns_message_parser::Dns;

use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::thread;

mod china_domain;

#[macro_use]
extern crate log;
extern crate dotenv;

use dotenv::dotenv;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::Duration;

static GLOBAL_DATA: Lazy<Mutex<china_domain::DomainCache>> = Lazy::new(|| {
    // Read Domain Cache File Content
    let cache = china_domain::DomainCache::init(None).unwrap();
    Mutex::new(cache)
});

fn main() {
    env_logger::init();
    dotenv().ok();

    let socket = UdpSocket::bind(std::env::var("LISTEN").unwrap()).unwrap();

    thread::spawn(|| loop {
        thread::sleep(Duration::from_secs(5));
        let cache = GLOBAL_DATA.lock().unwrap();
        cache.sync_to_file().unwrap();
    });

    loop {
        let mut buf = [0; 1024];
        let (size, origin) = socket.recv_from(&mut buf).unwrap();
        let socket_clone = socket.try_clone().unwrap();

        thread::spawn(
            move || match handle_request(&buf[..size], origin, &socket_clone) {
                Ok(_) => (),
                Err(e) => error!("{:?}", e),
            },
        );
    }
}

fn handle_request(
    msg: &[u8],
    from: SocketAddr,
    socket: &UdpSocket,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = Bytes::copy_from_slice(msg);

    let dns = Dns::decode(bytes.clone())?;

    let question = dns.questions.get(0).ok_or(Error::new(
        ErrorKind::InvalidData,
        "The Request dose not contain any question",
    ))?;

    let domain_name = question.domain_name.to_string();

    info!("Receive Query Domain {}", domain_name);

    let dns_server = if china_domain::is_china_domain(&domain_name)? {
        "https://dns.alidns.com"
    } else {
        "https://cloudflare-dns.com"
    };

    let client = reqwest::blocking::Client::new();

    let mut resp = client
        .post(format!("{}{}", dns_server, "/dns-query"))
        .header("content-type", "application/dns-message")
        .header("accept", "application/dns-message")
        .body(bytes.clone())
        .send()?;

    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf)?;
    socket.send_to(&buf, from)?;
    Ok(())
}

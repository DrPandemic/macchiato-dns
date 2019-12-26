extern crate nix;
extern crate tokio;

use std::str;
use tokio::net::UdpSocket;

pub mod network;
use crate::network::*;

const DEFAULT_DNS_RESOLVER: &str = "8.8.8.8:53";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:4053";

#[tokio::main]
async fn main() {
    let local_address = find_private_ipv4_address()
        .expect("couldn't find local address");
    let mut socket = UdpSocket::bind(DEFAULT_INTERNAL_ADDRESS).await
        .expect("tried to bind an UDP port");

    tokio::spawn(async move {
        loop {
            // Receives a single datagram message on the socket. If `buf` is too small to hold
            // the message, it will be cut off.
            // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
            let (buf, src) = recv_datagram(&mut socket).await
                .expect("couldn't receive datagram");
            let mut remote_socket = UdpSocket::bind((local_address, 0)).await
                .expect("couldn't bind remote resolver to address");
            remote_socket.send_to(&buf[..], DEFAULT_DNS_RESOLVER).await
                .expect("couldn't send data to remote");
            let (remote_buf, _) = recv_datagram(&mut remote_socket).await
                .expect("couldn't receive datagram from remote");
            socket.send_to(&remote_buf[..], &src).await
                .expect("failed to send to local socket");
        }
    }).await.unwrap();
}


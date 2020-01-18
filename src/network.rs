use tokio::net::UdpSocket;
use std::net::{Ipv4Addr};
use crate::message::*;

const MAX_DATAGRAM_SIZE: usize = 512;

pub async fn recv_datagram(socket: &mut UdpSocket) -> Option<(Vec<u8>, std::net::SocketAddr)> {
    let mut buf = [0; MAX_DATAGRAM_SIZE];
    let (amt, src) = match socket.recv_from(&mut buf).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("failed to read from local socket; err = {:?}", e);
            return None;
        }
    };
    Some((buf[..amt].to_vec(), src))
}

pub async fn receive_local_request(local_socket: &mut UdpSocket) -> (Message, std::net::SocketAddr) {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
    let (buf, src) = recv_datagram(local_socket).await
        .expect("couldn't receive datagram");
    println!("Q buffer: {:?}", buf);
    let message = parse_message(buf);
    let question = message.question().expect("couldn't parse question");
    println!("Q name: {:?} {:?}", question.qname().join("."), question.get_type());

    (message, src)
}

pub fn find_private_ipv4_address() -> Option<Ipv4Addr> {
    nix::ifaddrs::getifaddrs()
        .and_then(|addrs| Ok(
            addrs.filter_map(|addr| {
                if let Some(nix::sys::socket::SockAddr::Inet(address)) = addr.address {
                    if let std::net::IpAddr::V4(ip) = address.ip().to_std() {
                        Some(ip)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }).find(|ip| ip.is_private())
        ))
        .unwrap_or(None)
}

pub async fn query_remote_dns_server_udp(local_address: Ipv4Addr, remote_address: &str, query: Message) -> Message {
    let mut remote_socket = UdpSocket::bind((local_address, 0)).await
        .expect("couldn't bind remote resolver to address");
    query.send_to(&mut remote_socket, remote_address).await
        .expect("couldn't send data to remote");
    let (remote_buf, _) = recv_datagram(&mut remote_socket).await
        .expect("couldn't receive datagram from remote");
    println!("A buffer: {:?}", remote_buf);
    parse_message(remote_buf)
}

pub async fn query_remote_dns_server_doh(remote_address: &str, query: Message) -> Result<Message, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client.post(remote_address)
        .body(query.buffer)
        .header("content-type", "application/dns-udpwireformat")
        .send()
        .await?
        .bytes()
        .await?;

    Ok(parse_message(res.to_vec()))
}

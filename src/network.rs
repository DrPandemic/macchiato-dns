use tokio::net::UdpSocket;
use std::net::{Ipv4Addr};

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

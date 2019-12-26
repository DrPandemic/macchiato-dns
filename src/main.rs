extern crate nix;

use std::net::{Ipv4Addr, UdpSocket};
use std::str;
use nix::sys::socket::{InetAddr, SockAddr};

const MAX_DATAGRAM_SIZE: usize = 512;
const DEFAULT_DNS_RESOLVER: &str = "8.8.8.8:53";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:4053";

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind(DEFAULT_INTERNAL_ADDRESS)
        .expect("tried to bind an UDP port");

    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
    let mut buf = [0; MAX_DATAGRAM_SIZE];
    let (amt, src) = socket.recv_from(&mut buf)?;

    // Redeclare `buf` as slice of the received data and send reverse data back to origin.
    let buf = &buf[..amt];
    println!("stub {:?} {:?}", src, buf);

    {
        let local_address = find_private_ipv4_address()
            .expect("couldn't find local address");

        let remote_socket = UdpSocket::bind((local_address, 0))
            .expect("couldn't bind remote resolver to address");
        remote_socket
            .send_to(buf, DEFAULT_DNS_RESOLVER)
            .expect("couldn't send data to remote");
        let mut remote_buf = [0; MAX_DATAGRAM_SIZE];
        let (remote_amt, remote_src) = remote_socket.recv_from(&mut remote_buf)?;
        let remote_buf = &remote_buf[..remote_amt];
        println!("remote {:?} {:?}", remote_src, remote_buf);

        socket.send_to(remote_buf, &src)?;
    }
    Ok(())
}

fn find_private_ipv4_address() -> Option<Ipv4Addr> {
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

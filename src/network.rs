use crate::filter::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tokio::net::udp::RecvHalf;
use tokio::net::UdpSocket;

const MAX_DATAGRAM_SIZE: usize = 512;
const DEFAULT_DOH_DNS_RESOLVER: &str = "https://1.1.1.1/dns-query";

pub async fn recv_datagram(
    socket: &mut RecvHalf,
) -> Result<(Vec<u8>, std::net::SocketAddr), Box<dyn Error>> {
    let mut buf = [0; MAX_DATAGRAM_SIZE];
    let (amt, src) = socket.recv_from(&mut buf).await?;
    Ok((buf[..amt].to_vec(), src))
}

pub async fn receive_local_request(
    local_socket: &mut RecvHalf,
    verbose: u8,
) -> Result<(Message, std::net::SocketAddr), Box<dyn Error>> {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
    let (buf, src) = recv_datagram(local_socket).await?;
    if verbose > 2 {
        println!("Q buffer: {:?}", buf);
    }
    let message = parse_message(buf);
    // let question = message.question().expect("couldn't parse question");
    // println!("Q name: {:?} {:?}", question.qname().join("."), question.get_type());

    Ok((message, src))
}

pub fn find_private_ipv4_address() -> Option<Ipv4Addr> {
    nix::ifaddrs::getifaddrs()
        .and_then(|addrs| {
            Ok(addrs
                .filter_map(|addr| {
                    if let Some(nix::sys::socket::SockAddr::Inet(address)) = addr.address {
                        if let std::net::IpAddr::V4(ip) = address.ip().to_std() {
                            Some(ip)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .find(|ip| ip.is_private()))
        })
        .unwrap_or(None)
}

pub async fn query_remote_dns_server_udp(
    local_address: Ipv4Addr,
    remote_address: &SocketAddr,
    query: Message,
) -> Message {
    let remote_socket = UdpSocket::bind((local_address, 0))
        .await
        .expect("couldn't bind remote resolver to address");
    let (mut receiving, mut send) = remote_socket.split();
    query
        .send_to(&mut send, remote_address)
        .await
        .expect("couldn't send data to remote");
    let (remote_buf, _) = recv_datagram(&mut receiving)
        .await
        .expect("couldn't receive datagram from remote");
    // println!("A buffer: {:?}", remote_buf);
    parse_message(remote_buf)
}

pub async fn query_remote_dns_server_doh(
    remote_address: &str,
    query: Message,
) -> Result<Message, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .post(remote_address)
        .body(query.buffer)
        .header("content-type", "application/dns-udpwireformat")
        .send()
        .await?
        .bytes()
        .await?;

    Ok(parse_message(res.to_vec()))
}

pub async fn spawn_remote_dns_query(
    filter: Arc<Mutex<Filter>>,
    query: Message,
    src: SocketAddr,
    verbosity: u8,
    instrumentation: Instrumentation,
    response_sender: Sender<(SocketAddr, Instrumentation, Message)>,
) {
    let mut instrumentation = instrumentation;
    tokio::spawn(async move {
        let remote_answer = if filter_query(filter, &query, verbosity) {
            generate_deny_response(&query)
        } else {
            // query_remote_dns_server_udp(local_address, DEFAULT_DNS_RESOLVER, query).await
            instrumentation.set_request_sent();
            if let Ok(result) = query_remote_dns_server_doh(DEFAULT_DOH_DNS_RESOLVER, query).await {
                instrumentation.set_request_received();
                result
            } else {
                return log_error("Failed to send DoH", verbosity);
            }
        };

        // let answer_rrs = remote_answer.resource_records().expect("couldn't parse RRs");

        // println!("A data: {:?}", answer_rrs.0.into_iter().map(|rr| rr.name.join(".")).collect::<Vec<String>>());
        if response_sender
            .send((src, instrumentation, remote_answer))
            .is_err()
        {
            log_error("Failed to send a message on channel", verbosity);
        }
    })
    .await
    .unwrap();
}

fn filter_query(filter: Arc<Mutex<Filter>>, query: &Message, verbosity: u8) -> bool {
    if let Some(question) = query.question() {
        let name = question.qname().join(".");
        if verbosity > 1 {
            println!("{}", name);
        }
        let filter = filter.lock().unwrap();
        if filter.is_filtered(&name) {
            if verbosity > 0 {
                println!("{:?} was filtered!", name);
            }
            true
        } else {
            false
        }
    } else {
        log_error("couldn't parse question", verbosity);
        false
    }
}

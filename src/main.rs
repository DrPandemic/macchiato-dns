extern crate nix;
extern crate tokio;

use std::str;
use std::net::{Ipv4Addr};
use tokio::net::UdpSocket;

pub mod network;
pub mod message;
pub mod filter;
pub mod resource_record;
pub mod helpers;
pub mod question;
use crate::message::*;
use crate::network::*;
use crate::filter::*;

const DEFAULT_DNS_RESOLVER: &str = "8.8.8.8:53";
// const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:5553";

#[tokio::main]
async fn main() {
    let filter = Filter::from_disk(BlockFileVersion::Ultimate, FilterFormat::Hash).expect("Couldn't load filter");
    let local_address = find_private_ipv4_address()
        .expect("couldn't find local address");
    let mut socket = UdpSocket::bind(DEFAULT_INTERNAL_ADDRESS).await
        .expect("tried to bind an UDP port");

    tokio::spawn(async move {
        loop {
            let (query, src) = receive_local(&mut socket).await;
            let remote_answer = if filter_query(&filter, &query) {
                println!("This was filtered!");
                generate_deny_response(&query)
            } else {
                query_remote_dns_server(local_address, query).await
            };

            let answer_rrs = remote_answer.resource_records().expect("couldn't parse RRs");

            println!("A data: {:?}", answer_rrs.0.into_iter().map(|rr| rr.name.join(".")).collect::<Vec<String>>());
            remote_answer.send_to(&mut socket, &src).await
                .expect("failed to send to local socket");
        }
    }).await.unwrap();
}

async fn receive_local(local_socket: &mut UdpSocket) -> (Message, std::net::SocketAddr) {
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

async fn query_remote_dns_server(local_address: Ipv4Addr, query: Message) -> Message {
    let mut remote_socket = UdpSocket::bind((local_address, 0)).await
        .expect("couldn't bind remote resolver to address");
    query.send_to(&mut remote_socket, DEFAULT_DNS_RESOLVER).await
        .expect("couldn't send data to remote");
    let (remote_buf, _) = recv_datagram(&mut remote_socket).await
        .expect("couldn't receive datagram from remote");
    println!("A buffer: {:?}", remote_buf);
    parse_message(remote_buf)
}

fn filter_query(filter: &Filter, query: &Message) -> bool {
    let name = query.question().expect("couldn't parse question").qname();
    filter.is_filtered(name.join("."))
}

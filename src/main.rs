extern crate nix;
extern crate tokio;
extern crate reqwest;

use std::str;
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
const DEFAULT_DOH_DNS_RESOLVER: &str = "https://1.1.1.1/dns-query";
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
            let (query, src) = receive_local_request(&mut socket).await;
            let remote_answer = if filter_query(&filter, &query) {
                println!("This was filtered!");
                generate_deny_response(&query)
            } else {
                // query_remote_dns_server_udp(local_address, DEFAULT_DNS_RESOLVER, query).await
                query_remote_dns_server_doh(DEFAULT_DOH_DNS_RESOLVER, query).await.expect("couldn't parse doh answer")
            };

            let answer_rrs = remote_answer.resource_records().expect("couldn't parse RRs");

            println!("A data: {:?}", answer_rrs.0.into_iter().map(|rr| rr.name.join(".")).collect::<Vec<String>>());
            remote_answer.send_to(&mut socket, &src).await
                .expect("failed to send to local socket");
        }
    }).await.unwrap();
}

fn filter_query(filter: &Filter, query: &Message) -> bool {
    let name = query.question().expect("couldn't parse question").qname();
    filter.is_filtered(name.join("."))
}

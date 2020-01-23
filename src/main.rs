extern crate nix;
extern crate tokio;
extern crate reqwest;
extern crate cuckoofilter;

use std::str;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use structopt::StructOpt;

pub mod network;
pub mod message;
pub mod filter;
pub mod resource_record;
pub mod helpers;
pub mod question;
pub mod instrumentation;
pub mod cli;
use crate::message::*;
use crate::network::*;
use crate::filter::*;
use crate::instrumentation::*;
use crate::cli::*;

// const DEFAULT_DNS_RESOLVER: &str = "8.8.8.8:53";
const DEFAULT_DOH_DNS_RESOLVER: &str = "https://1.1.1.1/dns-query";
const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:5553";

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let verbose = opt.verbose;
    let filter_version = match &opt.filter_list[..] {
        "none" => FilterVersion::None,
        "blugo" => FilterVersion::Blugo,
        "ultimate" => FilterVersion::Ultimate,
        _ => panic!("Filter list is not valid"),
    };
    let filter_format = if opt.small {
        FilterFormat::Vector
    } else {
        FilterFormat::Hash
    };
    let filter = Arc::new(Mutex::new(Filter::from_disk(filter_version, filter_format).expect("Couldn't load filter")));
    let socket = UdpSocket::bind(if opt.debug { DEFAULT_INTERNAL_ADDRESS_DEBUG } else { DEFAULT_INTERNAL_ADDRESS }).await
        .expect("tried to bind an UDP port");
    let (mut receiving, mut sending) = socket.split();
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();

    tokio::spawn(async move {
        loop {
            let (src, instrumentation, message) = response_receiver.recv()
                .expect("failed to receive a message on channel");
            message.send_to(&mut sending, &src).await
                .expect("failed to send to local socket");
            if verbose > 1 {
                instrumentation.display();
            }
        }
    });

    loop {
        let (query, src) = receive_local_request(&mut receiving).await;
        let mut instrumentation = Instrumentation::new();
        let filter = Arc::clone(&filter);
        let response_sender = response_sender.clone();
        tokio::spawn(async move {
            let remote_answer = if filter_query(filter, &query, verbose > 0) {
                println!("This was filtered!");
                generate_deny_response(&query)
            } else {
                // query_remote_dns_server_udp(local_address, DEFAULT_DNS_RESOLVER, query).await
                instrumentation.set_request_sent();
                let result = query_remote_dns_server_doh(DEFAULT_DOH_DNS_RESOLVER, query).await.expect("couldn't parse doh answer");
                instrumentation.set_request_received();
                result
            };

            // let answer_rrs = remote_answer.resource_records().expect("couldn't parse RRs");

            // println!("A data: {:?}", answer_rrs.0.into_iter().map(|rr| rr.name.join(".")).collect::<Vec<String>>());
            response_sender.send((src, instrumentation, remote_answer))
                .expect("Failed to send a message on channel");
        }).await.unwrap();
    }
}

fn filter_query(filter: Arc<Mutex<Filter>>, query: &Message, log: bool) -> bool {
    let name = query.question().expect("couldn't parse question").qname().join(".");
    if log {
        println!("{}", name);
    }
    let filter = filter.lock().unwrap();
    filter.is_filtered(name)
}

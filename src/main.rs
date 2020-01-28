extern crate nix;
extern crate tokio;
extern crate reqwest;
extern crate cuckoofilter;

use std::str;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel};
use std::net::SocketAddr;
use std::path::PathBuf;
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
    let verbosity = opt.verbosity;
    let filter_version = match &opt.filter_list[..] {
        "none" => FilterVersion::None,
        "blu" => FilterVersion::Blu,
        "ultimate" => FilterVersion::Ultimate,
        _ => panic!("Filter list is not valid"),
    };
    let filter_format = if opt.small {
        FilterFormat::Vector
    } else {
        FilterFormat::Hash
    };
    let filters_path = opt.filters_path.unwrap_or(PathBuf::from("./"));

    let filter = Arc::new(Mutex::new(Filter::from_disk(filter_version, filter_format, filters_path).expect("Couldn't load filter")));
    let socket = UdpSocket::bind(if opt.debug { DEFAULT_INTERNAL_ADDRESS_DEBUG } else { DEFAULT_INTERNAL_ADDRESS }).await
        .expect("tried to bind an UDP port");
    let (mut receiving, mut sending) = socket.split();
    // TODO: Considere using https://docs.rs/async-std/1.3.0/async_std/sync/fn.channel.html
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();

    tokio::spawn(async move {
        loop {
            let result = response_receiver.recv();
            if let Ok((src, instrumentation, message)) = result {
                let sent = message.send_to(&mut sending, &src).await;
                if sent.is_err() {
                    log_error("Failed to send back UDP packet", verbosity);
                    continue
                }
                if verbosity > 2 {
                    instrumentation.display();
                }
            }
        }
    });

    loop {
        let local_result = receive_local_request(&mut receiving, verbosity).await;
        let (query, src) = match local_result {
            Ok(result) => result,
            _ => continue,
        };
        let mut instrumentation = Instrumentation::new();
        let filter = Arc::clone(&filter);
        let response_sender = response_sender.clone();
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
            if response_sender.send((src, instrumentation, remote_answer)).is_err() {
                log_error("Failed to send a message on channel", verbosity);
            }
        }).await.unwrap();
    }
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

fn log_error(message: &str, verbosity: u8) {
    if verbosity > 2 {
        println!("{:?}", message);
    }
}

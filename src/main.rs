extern crate lru;
extern crate nix;
extern crate reqwest;
extern crate tokio;

use std::net::SocketAddr;
use std::str;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio::net::UdpSocket;

pub mod cache;
pub mod cli;
pub mod filter;
pub mod helpers;
pub mod instrumentation;
pub mod message;
pub mod network;
pub mod question;
pub mod resource_record;
pub mod responder;
pub mod tree;
use crate::cache::*;
use crate::cli::*;
use crate::filter::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::network::*;
use crate::responder::*;

const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:5553";

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let verbosity = opt.verbosity;

    let filter = Arc::new(Mutex::new(Filter::from_opt(&opt)));
    let cache = Arc::new(Mutex::new(Cache::new()));
    let socket = UdpSocket::bind(if opt.debug {
        DEFAULT_INTERNAL_ADDRESS_DEBUG
    } else {
        DEFAULT_INTERNAL_ADDRESS
    })
    .await
    .expect("tried to bind an UDP port");
    let (mut receiving, sending) = socket.split();
    // TODO: Considere using https://docs.rs/async-std/1.3.0/async_std/sync/fn.channel.html
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();

    spawn_responder(sending, response_receiver, verbosity);

    loop {
        let (query, src) = match receive_local_request(&mut receiving, verbosity).await {
            Ok(result) => result,
            _ => continue,
        };
        spawn_remote_dns_query(
            Arc::clone(&filter),
            Arc::clone(&cache),
            query,
            src,
            verbosity,
            Instrumentation::new(),
            response_sender.clone(),
        );
    }
}

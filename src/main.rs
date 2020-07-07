#![allow(dead_code)]
extern crate lru;
extern crate nix;
extern crate reqwest;
extern crate smartstring;
extern crate tokio;

use std::net::SocketAddr;
use std::str;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio::net::UdpSocket;

mod cache;
mod cli;
mod dns_actors;
mod filter;
mod filter_statistics;
mod helpers;
mod instrumentation;
mod message;
mod network;
mod question;
mod resource_record;
mod tree;
mod web;
use crate::cache::*;
use crate::cli::*;
use crate::dns_actors::*;
use crate::filter::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::web::*;

const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:53";
const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:5553";

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let verbosity = opt.verbosity;

    let filter = Arc::new(Mutex::new(Filter::from_opt(&opt)));
    let cache = Arc::new(Mutex::new(Cache::new()));

    let socket = UdpSocket::bind(if opt.debug {
        DEFAULT_INTERNAL_ADDRESS_DEBUG
    } else if opt.external {
        DEFAULT_EXTERNAL_ADDRESS
    } else {
        DEFAULT_INTERNAL_ADDRESS
    })
    .await
    .expect("tried to bind an UDP port");
    let (receiving, sending) = socket.split();
    // TODO: Considere using https://docs.rs/async-std/1.3.0/async_std/sync/fn.channel.html
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();

    spawn_responder(sending, response_receiver, verbosity);
    spawn_listener(
        receiving,
        response_sender,
        Arc::clone(&filter),
        Arc::clone(&cache),
        verbosity,
    );
    start_web(&opt, filter, cache).await.unwrap();
}

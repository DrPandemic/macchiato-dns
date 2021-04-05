#![allow(dead_code)]
extern crate lru;
extern crate nix;
extern crate reqwest;
extern crate smartstring;
extern crate tokio;
extern crate toml;

use std::str;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio::net::UdpSocket;

mod cache;
mod cli;
mod config;
mod dns_actors;
mod filter;
mod filter_statistics;
mod helpers;
mod instrumentation;
mod message;
mod network;
mod question;
mod resolver_manager;
mod resource_record;
mod ring_buffer;
mod tree;
mod web;
mod web_auth;
use crate::cache::*;
use crate::cli::*;
use crate::config::Config;
use crate::dns_actors::*;
use crate::filter::*;
use crate::instrumentation::*;
use crate::resolver_manager::ResolverManager;
use crate::web::*;

const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_EXTERNAL_ADDRESS: &str = "0.0.0.0:53";
const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:5553";

#[tokio::main]
async fn main() {
    let config = Config::from_opt(Opt::from_args()).expect("Failed to read configuration file");
    let verbosity = config.verbosity;
    let socket = UdpSocket::bind(if config.debug {
        DEFAULT_INTERNAL_ADDRESS_DEBUG
    } else if config.external {
        DEFAULT_EXTERNAL_ADDRESS
    } else {
        DEFAULT_INTERNAL_ADDRESS
    })
    .await
    .expect("tried to bind an UDP port");
    let config = Arc::new(Mutex::new(config));

    let filter = Arc::new(Mutex::new(Filter::from_config(Arc::clone(&config))));
    let cache = Arc::new(Mutex::new(Cache::new()));
    let instrumentation_log = Arc::new(Mutex::new(InstrumentationLog::new()));
    let resolver_manager = Arc::new(Mutex::new(ResolverManager::new()));

    let (receiving, sending) = socket.split();

    let response_sender = spawn_responder(
        sending,
        Arc::clone(&instrumentation_log),
        Arc::clone(&resolver_manager),
        verbosity,
    );
    spawn_listener(
        receiving,
        response_sender,
        Arc::clone(&filter),
        Arc::clone(&cache),
        Arc::clone(&resolver_manager),
        Arc::clone(&config),
        verbosity,
    );
    let filter_update_channel = spawn_filter_updater(Arc::clone(&filter), Arc::clone(&config));
    start_web(Arc::clone(&config), filter, cache, instrumentation_log, filter_update_channel)
        .await
        .unwrap();
}

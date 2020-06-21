use crate::cache::*;
use crate::filter::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::network::*;

use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use tokio::net::udp::{RecvHalf, SendHalf};

pub fn spawn_responder(
    socket: SendHalf,
    channel: Receiver<(SocketAddr, Instrumentation, Message)>,
    verbosity: u8,
) {
    let mut socket = socket;
    tokio::spawn(async move {
        loop {
            let result = channel.recv();
            if let Ok((src, instrumentation, message)) = result {
                let sent = message.send_to(&mut socket, &src).await;
                if sent.is_err() {
                    log_error("Failed to send back UDP packet", verbosity);
                    continue;
                }
                if verbosity > 1 {
                    instrumentation.display();
                }
            }
        }
    });
}

pub fn spawn_listener(
    mut socket: RecvHalf,
    channel: Sender<(SocketAddr, Instrumentation, Message)>,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    verbosity: u8,
) {
    tokio::spawn(async move {
        loop {
            let (query, src) = match receive_local_request(&mut socket, verbosity).await {
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
                channel.clone(),
            );
        }
    });
}

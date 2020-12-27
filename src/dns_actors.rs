use crate::cache::*;
use crate::config::Config;
use crate::filter::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::network::*;
use crate::resolver_manager::ResolverManager;

use std::net::SocketAddr;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use tokio::net::udp::{RecvHalf, SendHalf};

pub fn spawn_responder(
    socket: SendHalf,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
    resolver_manager: Arc<Mutex<ResolverManager>>,
    verbosity: u8,
) -> Sender<(SocketAddr, Instrumentation, Message)> {
    // TODO: Considere using https://docs.rs/async-std/1.3.0/async_std/sync/fn.channel.html
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();
    let mut socket = socket;
    tokio::spawn(async move {
        loop {
            let result = response_receiver.recv();
            if let Ok((src, instrumentation, message)) = result {
                let sent = message.send_to(&mut socket, &src).await;
                if sent.is_err() {
                    log_error("Failed to send back UDP packet", verbosity);
                    continue;
                }
                if verbosity > 1 {
                    instrumentation.display();
                }
                let mut log = instrumentation_log.lock().unwrap();
                log.push(instrumentation);
                log.update_resolver_manager(Arc::clone(&resolver_manager));
            }
        }
    });

    return response_sender;
}

pub fn spawn_listener(
    mut socket: RecvHalf,
    channel: Sender<(SocketAddr, Instrumentation, Message)>,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    resolver_manager: Arc<Mutex<ResolverManager>>,
    config: Arc<Mutex<Config>>,
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
                Arc::clone(&resolver_manager),
                Arc::clone(&config),
                query,
                src,
                verbosity,
                Instrumentation::new(),
                channel.clone(),
            );
        }
    });
}

pub fn spawn_filter_updater(
    filter: Arc<Mutex<Filter>>,
    config: Arc<Mutex<Config>>,
) -> Sender<()> {
    let (response_sender, response_receiver) = channel::<()>();
    tokio::spawn(async move {
        loop {
            if response_receiver.recv().is_ok() {
                if let Ok(new_filter) = Filter::from_internet(Arc::clone(&config)).await {
                    let mut filter = filter.lock().unwrap();
                    *filter = new_filter;
                }
            } else {
                let config = config.lock().unwrap();
                log_error("Failed to send back UDP packet", config.verbosity);
            }
        }
    });

    return response_sender;
}

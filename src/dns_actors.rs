use crate::cache::*;
use crate::config::Config;
use crate::filter::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::network::*;
use crate::resolver_manager::ResolverManager;

use std::cmp;
use std::{net::SocketAddr, time::Duration};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use tokio::time::sleep;

pub fn spawn_responder(
    socket: Socket,
    instrumentation_log: Arc<Mutex<InstrumentationLog>>,
    resolver_manager: Arc<Mutex<ResolverManager>>,
    config: Arc<Mutex<Config>>,
    verbosity: u8,
) -> Sender<(SocketAddr, Instrumentation, Message)> {
    // TODO: Considere using https://docs.rs/async-std/1.3.0/async_std/sync/fn.channel.html
    let (response_sender, response_receiver) = channel::<(SocketAddr, Instrumentation, Message)>();
    tokio::spawn(async move {
        loop {
            let result = response_receiver.recv();
            if let Ok((src, instrumentation, message)) = result {
                {

                    let sent = message.send_to(socket.clone(), &src).await;
                    if sent.is_err() {
                        log_error("Failed to send back UDP packet", verbosity);
                        continue;
                    }
                }
                if verbosity > 1 {
                    instrumentation.display();
                }
                let mut log = instrumentation_log.lock().unwrap();
                log.push(instrumentation);
                log.update_resolver_manager(Arc::clone(&resolver_manager));
            } else {
                let config = config.lock().unwrap();
                log_error("Failed to receive message to responder", config.verbosity);
                if config.server_closing {
                    break;
                }
            }
        }
    });

    response_sender
}

pub fn spawn_listener(
    socket: Socket,
    channel: Sender<(SocketAddr, Instrumentation, Message)>,
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    resolver_manager: Arc<Mutex<ResolverManager>>,
    config: Arc<Mutex<Config>>,
    verbosity: u8,
) {
    tokio::spawn(async move {
        loop {
            let (query, src) = match receive_local_request(socket.clone(), verbosity).await {
                Ok(result) => result,
                Err(e) => {
                    log_error(format!("failed with {:?}", e).as_str(), verbosity);
                    continue
                },
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
                let result = Filter::from_internet(Arc::clone(&config)).await;

                match result {
                    Ok(new_filter) => {
                        let mut filter = filter.lock().unwrap();
                        *filter = new_filter;
                    },
                    Err(err) => {
                        let config = config.lock().unwrap();
                        let message = format!("{}", err);
                        log_error(&message, config.verbosity);
                    }
                }
            } else {
                let config = config.lock().unwrap();
                log_error("Failed to receive message to update filter", config.verbosity);
                if config.server_closing {
                    break;
                }
            }
        }
    });

    response_sender
}

pub fn spawn_filter_updater_ticker(
    config: Arc<Mutex<Config>>,
    filter_update_channel: Arc<Mutex<Sender<()>>>,
) {
    tokio::spawn(async move {
        loop {
            if config.lock().unwrap().auto_update.is_none() {
                sleep(Duration::from_secs(60 * 60)).await;
            } else {
                if filter_update_channel.lock().unwrap().send(()).is_err() {
                    return;
                }

                if config.lock().unwrap().auto_update.is_none() {
                    continue;
                }

                let auto_update = config.lock().unwrap().auto_update.unwrap();
                sleep(Duration::from_secs(cmp::max(60 * 60 * auto_update, 60 * 60))).await;
            }
        }
    });
}

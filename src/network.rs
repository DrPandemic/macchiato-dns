use crate::cache::*;
use crate::config::Config;
use crate::filter::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use crate::resolver_manager::ResolverManager;

use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

const MAX_DATAGRAM_SIZE: usize = 512;
pub type Socket = Arc<UdpSocket>;

pub async fn recv_datagram(socket: Socket) -> Result<(Vec<u8>, std::net::SocketAddr), Box<dyn Error>> {
    let mut buf = [0; MAX_DATAGRAM_SIZE];
    let (amt, src) = socket.recv_from(&mut buf).await?;

    Ok((buf[..amt].to_vec(), src))
}

pub async fn receive_local_request(
    local_socket: Socket,
    verbose: u8,
) -> Result<(Message, std::net::SocketAddr), Box<dyn Error>> {
    // Receives a single datagram message on the socket. If `buf` is too small to hold
    // the message, it will be cut off.
    // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
    let (buf, src) = recv_datagram(local_socket).await?;
    if verbose > 2 {
        println!("Q buffer: {:?}", buf);
    }
    let message = parse_message(buf);
    // let question = message.question().expect("couldn't parse question");
    // println!("Q name: {:?} {:?}", question.qname().join("."), question.get_type());

    Ok((message, src))
}

pub async fn query_remote_dns_server_udp(
    local_address: Ipv4Addr,
    remote_address: &SocketAddr,
    query: Message,
) -> Message {
    let remote_socket = UdpSocket::bind((local_address, 0))
        .await
        .expect("couldn't bind remote resolver to address");
    let remote_socket = Arc::new(remote_socket);
    query
        .send_to(remote_socket.clone(), remote_address)
        .await
        .expect("couldn't send data to remote");
    let (remote_buf, _) = recv_datagram(remote_socket.clone())
        .await
        .expect("couldn't receive datagram from remote");
    parse_message(remote_buf)
}

pub async fn query_remote_dns_server_doh(
    resolver: (String, Option<(&'static str, &'static str)>),
    query: Message,
) -> Result<Message, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new()
        .post(&resolver.0[..])
        .body(query.buffer)
        .header("content-type", "application/dns-message");

    let client = if let Some(header) = resolver.1 {
        client.header(header.0, header.1)
    } else {
        client
    };

    let res = client.send().await?.bytes().await?;

    Ok(parse_message(res.to_vec()))
}

pub fn spawn_remote_dns_query(
    filter: Arc<Mutex<Filter>>,
    cache: Arc<Mutex<Cache>>,
    resolver_manager: Arc<Mutex<ResolverManager>>,
    config: Arc<Mutex<Config>>,
    query: Message,
    src: SocketAddr,
    verbosity: u8,
    instrumentation: Instrumentation,
    response_sender: Sender<(SocketAddr, Instrumentation, Message)>,
) {
    let mut instrumentation = instrumentation;

    tokio::spawn(async move {
        let cached = if let Ok(mut cache) = cache.lock() {
            cache.get(&query)
        } else {
            return;
        };
        let (should_cache, remote_answer) = if let Some((cached, cache_time_left)) = cached {
            if verbosity > 0 {
                println!("{:?} was served from cache", cached.name());
            }

            if cache_time_left.as_secs() < 30 {
                let cache = Arc::clone(&cache);
                tokio::spawn(async move {
                    let resolver = resolver_manager.lock().unwrap().get_resolver();
                    match query_remote_dns_server_doh(resolver, query).await {
                        Ok(result) => {
                            cache.lock().unwrap().put(&result);
                            if verbosity > 1 {
                                println!("{:?} was prefetched", result.name());
                            }
                        }
                        Err(err) => {
                            log_error(&format!("Failed to send DoH, {}", err), verbosity);
                        }
                    }
                });
            }

            (false, cached)
        } else {
            if let Some(message) = override_query(filter, config, &query, verbosity) {
                (false, message)
            } else {
                let resolver = resolver_manager.lock().unwrap().get_resolver();
                instrumentation.set_request_sent(resolver.0.clone());
                match query_remote_dns_server_doh(resolver, query).await {
                    Ok(result) => {
                        instrumentation.set_request_received();
                        (true, result)
                    }
                    Err(err) => {
                        return log_error(&format!("Failed to send DoH, {}", err), verbosity);
                    }
                }
            }
        };

        if should_cache {
            cache.lock().unwrap().put(&remote_answer);
        }

        if response_sender.send((src, instrumentation, remote_answer)).is_err() {
            log_error("Failed to send a message on channel", verbosity);
        }
    });
}

fn override_query(
    filter: Arc<Mutex<Filter>>,
    config: Arc<Mutex<Config>>,
    query: &Message,
    verbosity: u8
) -> Option<Message> {
    if let Ok(question) = query.question() {
        let qname = question.qname();
        if qname.is_err() {
            return generate_deny_response(&query).ok()
        }
        let name: String = qname.unwrap().join(".").into();
        if verbosity > 1 {
            println!("{:?}", name);
        }

        {
            let overrides = &config.lock().unwrap().overrides;
            if let Some(response) = overrides.get(&name) {
                if verbosity > 0 {
                    let address = response.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(".");
                    println!("{:?} was overwritten with {:?}!", name.clone(), address);
                }
                return generate_override_response(&query, response)
                    .map_err(|err| {
                        log_error(&format!("Failed to generate override, {}", err), verbosity);
                        err
                    }).ok()
            }
        }

        let mut filter = filter.lock().unwrap();
        if let Some(filtered) = filter.filtered_by(&(name.clone().into()), &config.lock().unwrap().allowed_domains) {
            if verbosity > 0 {
                println!("{:?} was filtered by {:?}!", name, filtered);
            }
            generate_deny_response(&query).ok()
        } else {
            None
        }
    } else {
        log_error("couldn't parse question", verbosity);
        None
    }
}

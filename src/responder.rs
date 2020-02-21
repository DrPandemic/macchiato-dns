use crate::helpers::*;
use crate::instrumentation::*;
use crate::message::*;
use std::net::SocketAddr;
use std::sync::mpsc::Receiver;
use tokio::net::udp::SendHalf;

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
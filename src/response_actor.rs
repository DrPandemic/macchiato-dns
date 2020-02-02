use crate::dns_message::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::network::*;
use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::udp::SendHalf;

const DEFAULT_DOH_DNS_RESOLVER: &str = "https://1.1.1.1/dns-query";

pub struct ResponseActor {
    pub socket: Arc<Mutex<SendHalf>>,
    pub verbosity: u8,
}

impl Actor for ResponseActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendBackDnsResponse(pub DnsMessage, pub SocketAddr, pub Instrumentation);

impl Handler<SendBackDnsResponse> for ResponseActor {
    type Result = ();

    fn handle(&mut self, message: SendBackDnsResponse, _ctx: &mut Context<Self>) -> Self::Result {
        let SendBackDnsResponse(dns_message, addr, instrumentation);
        let verbosity = self.verbosity;
        let socket = Arc::clone(&self.socket);
        println!("a");

        actix_rt::spawn(async move {
            println!("b");
            let mut socket = socket.lock().unwrap();
            let SendBackDnsResponse(dns_message, addr, instrumentation) = message;
            let sent = dns_message.send_to(&mut socket, &addr).await;
            if sent.is_err() {
                log_error("Failed to send back UDP packet", verbosity);
            }
            if verbosity > 2 {
                instrumentation.display();
            }
        });
    }
}

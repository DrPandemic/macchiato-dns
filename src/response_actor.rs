use crate::cli::*;
use crate::dns_message::*;
use crate::helpers::*;
use crate::instrumentation::*;
use actix::prelude::*;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::udp::SendHalf;

pub struct ResponseActor {
    pub socket: Arc<Mutex<SendHalf>>,
    pub verbosity: u8,
}

impl ResponseActor {
    pub fn new(socket: SendHalf, opt: &Opt) -> ResponseActor {
        ResponseActor {
            verbosity: opt.verbosity,
            socket: Arc::new(Mutex::new(socket)),
        }
    }
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
        let verbosity = self.verbosity;
        let socket = Arc::clone(&self.socket);

        Arbiter::spawn(async move {
            let SendBackDnsResponse(dns_message, addr, instrumentation) = message;
            let sent = {
                let mut socket = socket.lock().unwrap();
                dns_message.send_to(&mut socket, &addr).await
            };
            if sent.is_err() {
                log_error("Failed to send back UDP packet", verbosity);
            }
            if verbosity > 2 {
                instrumentation.display();
            }
        });
    }
}

use crate::cli::*;
use crate::dns_message::*;
use crate::helpers::*;
use crate::instrumentation::*;
use crate::network::*;
use crate::response_actor::*;
use actix::prelude::*;
use std::net::SocketAddr;

const DEFAULT_DOH_DNS_RESOLVER: &str = "https://1.1.1.1/dns-query";

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendRemoteDnsQuery(pub DnsMessage, pub SocketAddr, pub Instrumentation);

pub struct RemoteDnsActor {
    pub verbosity: u8,
    pub response_actor: Addr<ResponseActor>,
}

impl RemoteDnsActor {
    pub fn new(response_actor: Addr<ResponseActor>, opt: &Opt) -> RemoteDnsActor {
        RemoteDnsActor {
            verbosity: opt.verbosity,
            response_actor: response_actor,
        }
    }
}

impl Actor for RemoteDnsActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut SyncContext<Self>) {}
}

impl Handler<SendRemoteDnsQuery> for RemoteDnsActor {
    type Result = ();

    fn handle(
        &mut self,
        message: SendRemoteDnsQuery,
        _ctx: &mut SyncContext<Self>,
    ) -> Self::Result {
        let SendRemoteDnsQuery(dns_message, addr, mut instrumentation) = message;
        instrumentation.set_request_sent();
        let verbosity = self.verbosity;
        let response_actor = self.response_actor.clone();

        Arbiter::spawn(async move {
            println!("a");
            if let Ok(result) =
                query_remote_dns_server_doh(DEFAULT_DOH_DNS_RESOLVER, dns_message).await
            {
                println!("b");
                instrumentation.set_request_received();
                response_actor.do_send(SendBackDnsResponse(result, addr, instrumentation));
            } else {
                return log_error("Failed to send DoH", verbosity);
            }
        });
    }
}

use actix::prelude::*;
use structopt::StructOpt;
use tokio::net::udp::RecvHalf;
use tokio::net::UdpSocket;

pub mod cli;
pub mod dns_message;
pub mod filter;
pub mod filter_actor;
pub mod helpers;
pub mod instrumentation;
pub mod network;
pub mod question;
pub mod remote_dns_actor;
pub mod resource_record;
pub mod response_actor;
use crate::cli::*;
use crate::filter_actor::*;
use crate::instrumentation::*;
use crate::network::*;
use crate::remote_dns_actor::*;
use crate::response_actor::*;

const DEFAULT_INTERNAL_ADDRESS: &str = "127.0.0.1:53";
const DEFAULT_INTERNAL_ADDRESS_DEBUG: &str = "127.0.0.1:5553";

fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    System::run(|| {
        let address = if opt.debug {
            DEFAULT_INTERNAL_ADDRESS_DEBUG
        } else {
            DEFAULT_INTERNAL_ADDRESS
        };

        Arbiter::spawn(async move {
            let socket = UdpSocket::bind(address)
                .await
                .expect("tried to bind an UDP port");

            let (receiving, sending) = socket.split();

            let response_actor = ResponseActor::new(sending, &opt).start();
            let response_actor2 = response_actor.clone();
            let opt2 = opt.clone();
            let remote_dns_actor =
                SyncArbiter::start(3, move || RemoteDnsActor::new(response_actor.clone(), &opt));

            let filter_actor = FilterActor::new(&opt2, response_actor2, remote_dns_actor).start();
            listen(receiving, opt2.verbosity, filter_actor).await;
        });
    })
}

async fn listen(mut socket: RecvHalf, verbosity: u8, filter_actor: Addr<FilterActor>) {
    loop {
        let local_result = receive_local_request(&mut socket, verbosity).await;
        let (query, src) = match local_result {
            Ok(result) => result,
            _ => continue,
        };

        filter_actor.do_send(DnsQueryReceived(query, src, Instrumentation::new()));
    }
}

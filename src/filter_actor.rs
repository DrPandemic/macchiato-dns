use crate::cli::*;
use crate::dns_message::*;
use crate::filter::*;
use crate::instrumentation::*;
use crate::response_actor::*;
use actix::prelude::*;
use std::net::SocketAddr;
use std::path::PathBuf;

pub struct FilterActor {
    pub filter: Filter,
    pub verbosity: u8,
    pub response_actor: Addr<ResponseActor>,
}

impl FilterActor {
    pub fn new(opt: &Opt, response_actor: Addr<ResponseActor>) -> FilterActor {
        let filter_version = match &opt.filter_list[..] {
            "none" => FilterVersion::None,
            "blu" => FilterVersion::Blu,
            "ultimate" => FilterVersion::Ultimate,
            _ => panic!("Filter list is not valid"),
        };
        let filter_format = if opt.small {
            FilterFormat::Vector
        } else {
            FilterFormat::Hash
        };
        let filters_path = opt.filters_path.clone().unwrap_or(PathBuf::from("./"));
        let filter = Filter::from_disk(filter_version, filter_format, filters_path)
            .expect("Failed to load filter from disk");
        let verbosity = opt.verbosity;
        FilterActor {
            filter: filter,
            verbosity: verbosity,
            response_actor: response_actor,
        }
    }
}

impl Actor for FilterActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DnsQueryReceived(pub DnsMessage, pub SocketAddr, pub Instrumentation);

impl Handler<DnsQueryReceived> for FilterActor {
    type Result = ();

    fn handle(&mut self, message: DnsQueryReceived, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(question) = message.0.question() {
            println!("foo");
            let name = question.qname().join(".");
            if self.verbosity > 1 {
                println!("{}", name);
            }
            if self.filter.is_filtered(&name) {
                if self.verbosity > 0 {
                    println!("{:?} was filtered!", name);
                }
                println!("asd");
                self.response_actor.do_send(SendBackDnsResponse(
                    generate_deny_response(&message.0),
                    message.1,
                    message.2,
                ));
            } else {
            }
        } else {
            log_error("couldn't parse question", self.verbosity);
        };
    }
}

fn log_error(message: &str, verbosity: u8) {
    if verbosity > 2 {
        println!("{:?}", message);
    }
}

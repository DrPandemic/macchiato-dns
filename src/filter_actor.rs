use crate::cli::*;
use crate::dns_message::*;
use crate::filter::*;
use actix::prelude::*;
use std::path::PathBuf;

#[derive(Default)]
pub struct FilterActor {
    pub filter: Filter,
    verbosity: u8,
}
impl actix::Supervised for FilterActor {}

impl ArbiterService for FilterActor {
    fn service_started(&mut self, _ctx: &mut Context<Self>) {}
}

impl Actor for FilterActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}
}

pub struct IsFiltered(pub DnsMessage);
pub struct ChangeFilter(pub Opt);

impl Message for IsFiltered {
    type Result = bool;
}

impl Message for ChangeFilter {
    type Result = ();
}

impl Handler<ChangeFilter> for FilterActor {
    type Result = ();

    fn handle(&mut self, opt: ChangeFilter, _ctx: &mut Context<Self>) -> Self::Result {
        let filter_version = match &opt.0.filter_list[..] {
            "none" => FilterVersion::None,
            "blu" => FilterVersion::Blu,
            "ultimate" => FilterVersion::Ultimate,
            _ => panic!("Filter list is not valid"),
        };
        let filter_format = if opt.0.small {
            FilterFormat::Vector
        } else {
            FilterFormat::Hash
        };
        let filters_path = opt.0.filters_path.clone().unwrap_or(PathBuf::from("./"));
        self.filter = Filter::from_disk(filter_version, filter_format, filters_path)
            .expect("Failed to load filter from disk");
        self.verbosity = opt.0.verbosity;
    }
}
impl Handler<IsFiltered> for FilterActor {
    type Result = bool;

    fn handle(&mut self, query: IsFiltered, _ctx: &mut Context<Self>) -> Self::Result {
        if let Some(question) = query.0.question() {
            let name = question.qname().join(".");
            if self.verbosity > 1 {
                println!("{}", name);
            }
            if self.filter.is_filtered(&name) {
                if self.verbosity > 0 {
                    println!("{:?} was filtered!", name);
                }
                true
            } else {
                false
            }
        } else {
            log_error("couldn't parse question", self.verbosity);
            false
        }
    }
}

fn log_error(message: &str, verbosity: u8) {
    if verbosity > 2 {
        println!("{:?}", message);
    }
}

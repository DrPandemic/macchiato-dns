use actix::prelude::*;
use structopt::StructOpt;

pub mod cli;
pub mod dns_message;
pub mod filter;
pub mod filter_actor;
pub mod helpers;
pub mod instrumentation;
pub mod network;
pub mod question;
pub mod resource_record;
use crate::cli::*;
use crate::dns_message::*;
use crate::filter_actor::*;
// use crate::instrumentation::*;
// use crate::network::*;

const IMATEAPOT_QUESTION: [u8; 46] = [
    57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97, 112,
    111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0,
];

fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    let system = System::new("macchiato");

    let addr = FilterActor::from_registry();
    let buffer = IMATEAPOT_QUESTION.to_vec();
    let message = parse_message(buffer);

    Arbiter::spawn(async move {
        addr.send(ChangeFilter(opt.clone()))
            .await
            .expect("Failed to change filter");
        println!("Changed");
        let result = addr.send(IsFiltered(message)).await;
        println!("{:?}", result);
    });

    system.run()
}

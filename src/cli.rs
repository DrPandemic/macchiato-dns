use std::path::PathBuf;
use structopt::StructOpt;

/// A simple DNS proxy server that that will protect your communications
#[derive(StructOpt, Debug)]
#[structopt(name = "Macchiato DNS")]
pub struct Opt {
    /// Activate debug mode. Runs server on 5553
    #[structopt(short, long)]
    pub debug: bool,

    /// Configuration file
    #[structopt(long, parse(from_os_str), default_value = "./config.toml")]
    pub configuration: PathBuf,
}

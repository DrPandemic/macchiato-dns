use structopt::StructOpt;
use std::path::PathBuf;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct Opt {
    /// Activate debug mode. Runs server on 5553
    #[structopt(short, long)]
    pub debug: bool,

    /// Verbosity level (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbosity: u8,

    /// Uses a smaller but slower data structure to keep domain filter list
    #[structopt(short, long)]
    pub small: bool,

    /// none, blu or ultimate. Defaults to blu
    #[structopt(short = "f", long, default_value = "blu")]
    pub filter_list: String,

    /// Directory containing filter lists
    #[structopt(long, parse(from_os_str))]
    pub filters_path: Option<PathBuf>,
}

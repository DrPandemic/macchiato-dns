use structopt::StructOpt;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct Opt {
    /// Activate debug mode. Runs server on 5553
    #[structopt(short, long)]
    pub debug: bool,

    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// Uses a smaller but slower data structure to keep domain filter list
    #[structopt(short, long)]
    pub small: bool,

    /// none, blu or ultimate. Defaults to blu
    #[structopt(short = "f", long, default_value = "blu")]
    pub filter_list: String,
}

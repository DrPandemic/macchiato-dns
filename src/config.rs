use crate::cli::Opt;
use crate::web_auth::get_web_password_hash;
use std::path::PathBuf;

pub struct Config {
    pub web_password_hash: String,
    pub debug: bool,
    pub verbosity: u8,
    pub small: bool,
    pub filter_list: String,
    pub filters_path: Option<PathBuf>,
    pub allowed_domains: Vec<String>,
    pub external: bool,
}

impl Config {
    pub fn from_opt(opt: Opt) -> Config {
        Config {
            web_password_hash: get_web_password_hash(&opt),
            debug: opt.debug,
            verbosity: opt.verbosity,
            small: opt.small,
            filter_list: opt.filter_list,
            filters_path: opt.filters_path,
            allowed_domains: opt.allowed.clone().unwrap_or(vec![]),
            external: opt.external,
        }
    }
}

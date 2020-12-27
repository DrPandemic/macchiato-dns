use crate::cli::Opt;
use crate::filter::{FilterVersion, FilterFormat};
use crate::web_auth::get_web_password_hash;
use std::path::PathBuf;

pub struct Config {
    pub allowed_domains: Vec<String>,
    pub debug: bool,
    pub external: bool,
    pub filters_path: Option<PathBuf>,
    pub filter_version: FilterVersion,
    pub filter_format: FilterFormat,
    pub small: bool,
    pub verbosity: u8,
    pub web_password_hash: String,
}

impl Config {
    pub fn from_opt(opt: Opt) -> Config {
        Config {
            web_password_hash: get_web_password_hash(&opt),
            debug: opt.debug,
            verbosity: opt.verbosity,
            small: opt.small,
            filter_version: Self::filter_version(&opt.filter_version),
            filter_format: Self::filter_format(&opt.small),
            filters_path: opt.filters_path,
            allowed_domains: opt.allowed.clone().unwrap_or(vec![]),
            external: opt.external,
        }
    }

    pub fn filter_version(filter_version: &String) -> FilterVersion {
        match &filter_version[..] {
            "none" => FilterVersion::None,
            "blu" => FilterVersion::Blu,
            "ultimate" => FilterVersion::Ultimate,
            "test" => FilterVersion::Test,
            _ => panic!("Filter list is not valid"),
        }
    }

    pub fn filter_format(small: &bool) -> FilterFormat {
        if *small {
            FilterFormat::Vector
        } else {
            FilterFormat::Tree
        }
    }
}

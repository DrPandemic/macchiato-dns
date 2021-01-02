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

impl Default for Config {
    fn default() -> Config {
        Config {
            allowed_domains: vec![],
            debug: true,
            external: true,
            filters_path: Some(PathBuf::from("./")),
            filter_version: FilterVersion::None,
            filter_format: FilterFormat::Tree,
            small: true,
            verbosity: 0,
            web_password_hash: String::from("asdf"),
        }
    }
}

impl Config {
    pub fn from_opt(opt: Opt) -> Config {
        let mut allowed = opt.allowed.clone().unwrap_or(vec![]);
        allowed.sort();
        Config {
            web_password_hash: get_web_password_hash(&opt),
            debug: opt.debug,
            verbosity: opt.verbosity,
            small: opt.small,
            filter_version: Self::filter_version(&opt.filter_version),
            filter_format: Self::filter_format(&opt.small),
            filters_path: opt.filters_path,
            allowed_domains: allowed,
            external: opt.external,
        }
    }

    fn filter_version(filter_version: &String) -> FilterVersion {
        match &filter_version[..] {
            "none" => FilterVersion::None,
            "blu" => FilterVersion::Blu,
            "ultimate" => FilterVersion::Ultimate,
            "test" => FilterVersion::Test,
            _ => panic!("Filter list is not valid"),
        }
    }

    fn filter_format(small: &bool) -> FilterFormat {
        if *small {
            FilterFormat::Vector
        } else {
            FilterFormat::Tree
        }
    }
}

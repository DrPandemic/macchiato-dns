use crate::cli::Opt;
use crate::filter::{FilterVersion, FilterFormat};
use crate::web_auth::get_web_password_hash;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub allowed_domains: Vec<String>,
    pub external: bool,
    pub filters_path: Option<PathBuf>,
    pub filter_version: FilterVersion,
    pub small: bool,
    pub verbosity: u8,

    pub web_password: Option<String>,

    #[serde(skip_deserializing, skip_serializing)]
    pub web_password_hash: String,

    #[serde(skip_deserializing, skip_serializing)]
    pub configuration_path: PathBuf,
    #[serde(skip_deserializing, skip_serializing)]
    pub debug: bool,
    #[serde(skip_deserializing, skip_serializing)]
    pub filter_format: FilterFormat,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            allowed_domains: vec![],
            debug: true,
            external: true,
            filters_path: Some(PathBuf::from("./")),
            filter_version: FilterVersion::Blu,
            filter_format: FilterFormat::Vector,
            small: true,
            verbosity: 0,
            web_password: None,
            web_password_hash: String::from("abcd"),
            configuration_path: PathBuf::from("./config.toml")
        }
    }
}

impl Config {
    pub fn from_opt(opt: Opt) -> Result<Config, std::io::Error> {
        let mut config_toml = String::new();
        let mut file = fs::File::open(&opt.configuration)?;
        file.read_to_string(&mut config_toml)?;

        let mut config: Config = toml::from_str(&config_toml)?;

        config.configuration_path = opt.configuration;
        config.debug = opt.debug;
        config.web_password_hash = get_web_password_hash(config.web_password.clone());
        config.filter_format = if config.small {
            FilterFormat::Vector
        } else {
            FilterFormat::Tree
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        Ok(fs::write(self.configuration_path.clone(), toml::to_string(&self)?)?)
    }
}

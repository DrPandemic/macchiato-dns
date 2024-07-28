use crate::cli::Opt;
use crate::filter::{FilterFormat, FilterVersion};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub allowed_domains: Vec<String>,
    pub auto_update: Option<u64>,
    pub external: bool,
    pub filters_path: Option<PathBuf>,
    pub filter_version: FilterVersion,
    pub small: bool,
    pub verbosity: u8,

    pub web_password: Option<String>,

    #[serde(skip_deserializing, skip_serializing)]
    pub configuration_path: PathBuf,
    #[serde(skip_deserializing, skip_serializing)]
    pub debug: bool,
    #[serde(skip_deserializing, skip_serializing)]
    pub filter_format: FilterFormat,
    #[serde(skip_deserializing, skip_serializing)]
    pub server_closing: bool,
    #[serde(skip_deserializing, skip_serializing)]
    pub disabled_until: u64,
    #[serde(skip_deserializing, skip_serializing)]
    #[serde(serialize_with = "toml::ser::tables_last")]
    pub overrides: HashMap<String, Vec<u8>>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            allowed_domains: vec![],
            auto_update: None,
            configuration_path: PathBuf::from("./config.toml"),
            debug: true,
            disabled_until: 0,
            external: true,
            filters_path: Some(PathBuf::from("./")),
            filter_format: FilterFormat::Vector,
            filter_version: FilterVersion::Blu,
            overrides: HashMap::default(),
            server_closing: false,
            small: true,
            verbosity: 0,
            web_password: None,
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

    pub fn disabled(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        self.disabled_until > now
    }
}

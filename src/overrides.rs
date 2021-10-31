use crate::config::Config;
// use smartstring::alias::String;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverrideContainer(HashMap<String, Vec<u8>>);

impl OverrideContainer {
    pub fn from_config(config: Arc<Mutex<Config>>) -> OverrideContainer {
        config.lock().unwrap().overrides.clone()
    }

    pub fn get(&self, name: &String) -> Option<&Vec<u8>> {
        self.0.get(name)
    }

    pub fn insert(&mut self, name: String, address: Vec<u8>) {
        self.0.insert(
            name.clone(),
            address,
        );
    }
}

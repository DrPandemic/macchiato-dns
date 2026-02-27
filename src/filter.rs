use crate::config::Config;
use crate::filter_statistics::*;
use crate::helpers::log_error;
use crate::tree::*;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, Read, empty};
use std::path::PathBuf;
use std::time::SystemTime;
use std::sync::{Arc, Mutex};
use std::fs;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FilterVersion {
    None,
    Blu,
    Ultimate,
    Test,
    OneHostsLite,
    OneHostsPro,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum FilterFormat {
    #[default]
    Vector,
    Hash,
    Tree,
}

pub struct Filter {
    pub format: FilterFormat,
    pub vector: Option<Vec<String>>,
    pub hash: Option<HashSet<String>>,
    pub tree: Option<Tree>,
    pub statistics: FilterStatistics,
    pub size: usize,
    pub created_at: SystemTime,
}

impl Default for Filter {
    fn default() -> Filter {
        Filter {
            format: FilterFormat::Vector,
            vector: None,
            hash: None,
            tree: None,
            statistics: FilterStatistics::new(),
            size: 0,
            created_at: SystemTime::now(),
        }
    }
}

impl Filter {
    pub fn from_config(config: Arc<Mutex<Config>>) -> Filter {
        let filters_path = config.lock().unwrap().filters_path.clone().unwrap_or(PathBuf::from("./"));

        Filter::from_disk(config.clone(), filters_path)
          .inspect_err(|_| log_error("Couldn't load filter from disk.", 3))
          .or(Self::from_buffer(config, io::BufReader::new(empty())))
          .expect("Couldn't load filter")
    }

    fn get_file_name(version: &FilterVersion) -> Option<String> {
        match version {
            FilterVersion::Blu => Some(String::from("blu.txt")),
            FilterVersion::Ultimate => Some(String::from("ultimate.txt")),
            FilterVersion::Test => Some(String::from("test_filter.txt")),
            FilterVersion::OneHostsLite => Some(String::from("1hosts_lite.txt")),
            FilterVersion::OneHostsPro => Some(String::from("1hosts_pro.txt")),
            FilterVersion::None => None,
        }
    }

    pub async fn from_internet(config: Arc<Mutex<Config>>) -> Result<Filter, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .gzip(true)
            .build()?;
        let response = client
          .get(get_download_url(Arc::clone(&config)))
          .send().await?
          .text().await?;
        let buffer = io::BufReader::new(response.as_bytes());

        Self::from_buffer(config, buffer).map_err(|e| e.into())
    }

    pub fn from_disk(config: Arc<Mutex<Config>>, path: PathBuf) -> Result<Filter, std::io::Error> {
        let filter_version = &config.lock().unwrap().filter_version.clone();
        if let Some(file_name) = Filter::get_file_name(filter_version) {
            let file = File::open(path.join(file_name.to_string()))?;
            Self::from_buffer(config, io::BufReader::new(file)).map(|mut filter| {
                if let Ok(Ok(time)) = fs::metadata(file_name.to_string()).map(|metadata| metadata.created()) {
                    filter.created_at = time;
                }
                filter
            })
        } else {
            Self::from_buffer(config, io::BufReader::new(empty()))
        }
    }

    fn from_buffer<T: Read>(config: Arc<Mutex<Config>>, buffer: io::BufReader<T>) -> Result<Filter, std::io::Error> {
        let mut lines = buffer.lines()
            .filter_map(|maybe_line| match maybe_line {
                Ok(line) => {
                    let line: String = line.into();
                    if line.starts_with("#") {
                        None
                    } else {
                        Some(line)
                    }
                }
                _ => None,
            })
            .collect::<Vec<String>>();
        lines.sort();

        let filter_format = config.lock().unwrap().filter_format.clone();
        match filter_format {
            FilterFormat::Vector => Ok(Filter {
                format: filter_format,
                size: lines.len(),
                vector: Some(lines),
                ..Default::default()
            }),
            FilterFormat::Hash => {
                let mut hash = HashSet::new();
                let len = lines.len();
                for line in lines {
                    hash.insert(line);
                }
                Ok(Filter {
                    format: filter_format,
                    size: len,
                    hash: Some(hash),
                    ..Default::default()
                })
            }
            FilterFormat::Tree => {
                let mut tree = Tree::new();
                let len = lines.len();
                for line in lines {
                    tree.insert(&line);
                }
                Ok(Filter {
                    format: filter_format,
                    size: len,
                    tree: Some(tree),
                    ..Default::default()
                })
            }
        }
    }

    pub fn filtered_by(&mut self, name: &String, config: &Config) -> Option<String> {
        if config.disabled() {
            return None;
        }
        if is_name_in_allowed_list(name, &config.allowed_domains) {
            return None;
        }
        let result = match self.format {
            FilterFormat::Vector => {
                let vector = self.vector.as_ref().unwrap();

                filtered_by(&name, |name| {
                    let result = vector.binary_search(name).ok();
                    result.and_then(|i| vector.get(i).map(|s| s.clone()))
                })
            }
            FilterFormat::Hash => {
                let hash = self.hash.as_ref().unwrap();

                filtered_by(&name, |name| hash.get(name).map(|name| name.clone()))
            }
            FilterFormat::Tree => self.tree.as_ref().unwrap().contains(name),
        };

        if let Some(filtered) = result {
            self.statistics.increment(&filtered);
            Some(filtered)
        } else {
            None
        }
    }
}

fn filtered_by(name: &String, contains: impl Fn(&String) -> Option<String>) -> Option<String> {
    let parts = name.split(".").collect::<Vec<&str>>();
    (0..parts.len()).find_map(|i| {
        let name = parts.get(i..parts.len()).unwrap().join(".").into();
        let result = contains(&name);
        result
    })
}

fn is_name_in_allowed_list(name: &String, allowed_domains: &Vec<std::string::String>) -> bool {
    filtered_by(&name, |name| {
        allowed_domains
            .binary_search(&std::string::String::from(name.as_str()))
            .ok()
            .map(|_| String::from(""))
    }).is_some()
}

fn get_download_url(config: Arc<Mutex<Config>>) -> &'static str {
    match config.lock().unwrap().filter_version {
        FilterVersion::Ultimate => "https://block.energized.pro/ultimate/formats/domains.txt",
        FilterVersion::OneHostsLite => "https://badmojr.gitlab.io/1hosts/Lite/domains.txt",
        FilterVersion::OneHostsPro => "https://badmojr.gitlab.io/1hosts/Pro/domains.txt",
        _ => "https://block.energized.pro/blu/formats/domains.txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_all_types() {
        vec![FilterFormat::Vector, FilterFormat::Hash, FilterFormat::Tree]
            .iter()
            .for_each(move |format| {
                let config = Arc::new(Mutex::new(Config{filter_version: FilterVersion::Test, filter_format: format.clone(), ..Default::default()}));
                let mut filter = Filter::from_disk(Arc::clone(&config), PathBuf::from("./"))
                    .expect("Couldn't load filter");

                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.org"), &*config.lock().unwrap())
                );

                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.org"), &*config.lock().unwrap())
                );
                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("m.www.imateapot.org"), &*config.lock().unwrap())
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.ca"), &*config.lock().unwrap()));
                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.info"), &*config.lock().unwrap())
                );
                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("m.www.imateapot.info"), &*config.lock().unwrap())
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.info"), &*config.lock().unwrap()));
                assert_eq!(None, filter.filtered_by(&String::from("org"), &*config.lock().unwrap()));
                assert_eq!(None, filter.filtered_by(&String::from("com"), &*config.lock().unwrap()));

                let allowed = vec![std::string::String::from("imateapot.org")];
                config.lock().unwrap().allowed_domains = allowed;

                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("imateapot.org"), &*config.lock().unwrap())
                );
                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.org"), &*config.lock().unwrap())
                );

                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.org"), &*config.lock().unwrap())
                );
                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("m.www.imateapot.org"), &*config.lock().unwrap())
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.ca"), &*config.lock().unwrap()));
                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.info"), &*config.lock().unwrap())
                );
                assert_ne!(
                    None,
                    filter.filtered_by(&String::from("m.www.imateapot.info"), &*config.lock().unwrap())
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.info"), &*config.lock().unwrap()));
                assert_eq!(None, filter.filtered_by(&String::from("org"), &*config.lock().unwrap()));
                assert_eq!(None, filter.filtered_by(&String::from("com"), &*config.lock().unwrap()));
            });
    }

    #[test]
    fn allowed() {
        vec![FilterFormat::Vector, FilterFormat::Hash, FilterFormat::Tree]
            .iter()
            .for_each(move |format| {
                let config = Arc::new(Mutex::new(Config{filter_version: FilterVersion::Test, filter_format: format.clone(), ..Default::default()}));
                let mut filter = Filter::from_disk(Arc::clone(&config), PathBuf::from("./"))
                    .expect("Couldn't load filter");
                let allowed = vec![
                    std::string::String::from("bar.com"),
                    std::string::String::from("foo.com"),
                    std::string::String::from("imateapot.org"),
                ];

                config.lock().unwrap().allowed_domains = allowed;

                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("imateapot.org"), &*config.lock().unwrap())
                );
                assert_eq!(
                    None,
                    filter.filtered_by(&String::from("www.imateapot.org"), &*config.lock().unwrap())
                );
            });
    }
}

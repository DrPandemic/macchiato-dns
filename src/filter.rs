use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use cuckoofilter::*;

pub enum FilterVersion {
    None,
    Blu,
    Ultimate,
}

pub enum FilterFormat {
    Vector,
    Hash,
    Bloom,
    Xor,
    Cuckoo,
}

pub struct Filter {
    pub format: FilterFormat,
    pub vector: Option<Vec<String>>,
    pub hash: Option<HashSet<String>>,
    pub cuckoo: Option<CuckooFilter<std::collections::hash_map::DefaultHasher>>,
}

impl Filter {
    // fn get_url(version: FilterVersion) -> Option<String> {
    //     match version {
    //         FilterVersion::Blugo => Some(String::from("https://block.energized.pro/bluGo/formats/domains.txt")),
    //         FilterVersion::Ultimate => Some(String::from("https://block.energized.pro/ultimate/formats/domains.txt")),
    //         FilterVersion::None => None
    //     }
    // }

    fn get_file_name(version: FilterVersion) -> Option<String> {
        match version {
            FilterVersion::Blu => Some(String::from("blu.txt")),
            FilterVersion::Ultimate => Some(String::from("ultimate.txt")),
            FilterVersion::None => None,
        }
    }


    pub fn from_download(_version: FilterVersion, _format: FilterFormat) -> Filter {
        Filter {
            format: FilterFormat::Vector,
            vector: Some(vec![]),
            hash: None,
            cuckoo: None,
        }
    }

    pub fn from_disk(version: FilterVersion, format: FilterFormat, path: PathBuf) -> Result<Filter, std::io::Error> {
        let lines = if let Some(file_name) = Filter::get_file_name(version) {
            let file = File::open(path.join(file_name))?;
            let mut vec = io::BufReader::new(file)
                .lines()
                .filter_map(|maybe_line| {
                    match maybe_line {
                        Ok(line) => if line.starts_with("#") { None } else { Some(line) }
                        _ => None
                    }
                })
                .collect::<Vec<String>>();
            vec.sort();
            vec
        } else {
            vec![]
        };

        match format {
            FilterFormat::Vector => {
                Ok(Filter {
                    format: format,
                    vector: Some(lines),
                    hash: None,
                    cuckoo: None,
                })
            },
            FilterFormat::Hash => {
                let mut hash = HashSet::new();
                for line in lines {
                    hash.insert(line);
                }
                Ok(Filter {
                    format: format,
                    vector: None,
                    hash: Some(hash),
                    cuckoo: None,
                })
            },
            FilterFormat::Cuckoo => {
                let mut filter = CuckooFilter::new();
                for line in lines.clone() {
                    filter.add(&line);
                }
                Ok(Filter {
                    format: format,
                    vector: Some(lines),
                    hash: None,
                    cuckoo: Some(filter),
                })
            }
            _ => panic!()
        }
    }

    pub fn is_filtered(&self, name: &String) -> bool {
        match self.format {
            FilterFormat::Vector => self.vector.as_ref().unwrap().binary_search(name).is_ok(),
            FilterFormat::Hash => self.hash.as_ref().unwrap().contains(name),
            FilterFormat::Cuckoo => {
                !(
                    !self.cuckoo.as_ref().unwrap().contains(name) &&
                        !self.vector.as_ref().unwrap().binary_search(name).is_ok()
                )
            },
            _ => false,
        }
    }
}

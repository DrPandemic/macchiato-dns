use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};

#[derive(Copy, Clone, Debug)]
pub enum BlockFileVersion {
    None,
    Blugo,
    Ultimate,
}

#[derive(Copy, Clone, Debug)]
pub enum FilterFormat {
    Vector,
    Hash,
    Bloom,
    Xor,
}

#[derive(Clone, Debug)]
pub struct Filter {
    pub format: FilterFormat,
    pub domains_vector: Option<Vec<String>>,
    pub domains_hash: Option<HashMap<String, u8>>
}

impl Filter {
    // fn get_url(version: BlockFileVersion) -> Option<String> {
    //     match version {
    //         BlockFileVersion::Blugo => Some(String::from("https://block.energized.pro/bluGo/formats/domains.txt")),
    //         BlockFileVersion::Ultimate => Some(String::from("https://block.energized.pro/ultimate/formats/domains.txt")),
    //         BlockFileVersion::None => None
    //     }
    // }

    fn get_file_name(version: BlockFileVersion) -> Option<String> {
        match version {
            BlockFileVersion::Blugo => Some(String::from("blugo.txt")),
            BlockFileVersion::Ultimate => Some(String::from("ultimate.txt")),
            BlockFileVersion::None => None
        }
    }


    pub fn from_download(_version: BlockFileVersion, _format: FilterFormat) -> Filter {
        Filter {
            format: FilterFormat::Vector,
            domains_vector: Some(vec![]),
            domains_hash: None,
        }
    }

    pub fn from_disk(version: BlockFileVersion, format: FilterFormat) -> Result<Filter, std::io::Error> {
        let lines = if let Some(file_name) = Filter::get_file_name(version) {
            let file = File::open(file_name)?;
            io::BufReader::new(file)
                .lines()
                .filter_map(|maybe_line| {
                    match maybe_line {
                        Ok(line) => if line.starts_with("#") { None } else { Some(line) }
                        _ => None
                    }
                })
                .collect::<Vec<String>>()
        } else {
            vec![]
        };

        match format {
            FilterFormat::Vector => {
                Ok(Filter {
                    format: FilterFormat::Vector,
                    domains_vector: Some(lines),
                    domains_hash: None,
                })
            },
            FilterFormat::Hash => {
                let mut hash = HashMap::new();
                for line in lines {
                    hash.insert(line, 0);
                }
                Ok(Filter {
                    format: FilterFormat::Hash,
                    domains_vector: None,
                    domains_hash: Some(hash),
                })
            },
            _ => panic!()
        }
    }

    pub fn is_filtered(&self, name: String) -> bool {
        match self.format {
            FilterFormat::Vector => self.domains_vector.as_ref().unwrap().iter().any(|line| line == &name),
            FilterFormat::Hash => self.domains_hash.as_ref().unwrap().contains_key(&name),
            _ => false,
        }
    }
}

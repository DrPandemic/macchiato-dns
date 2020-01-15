use std::fs::File;
use std::io::{self, BufRead};

pub enum BlockFileVersion {
    None,
    Blugo,
    Ultimate,
}

pub enum FilterFormat {
    Vector,
    Bloom,
    Xor,
}

pub struct Filter {
    pub format: FilterFormat,
    pub domains_vector: Option<Vec<String>>,
}

const forbidden: [&str; 3] = ["avatars3", "githubusercontent", "com"];

impl Filter {
    fn get_url(version: BlockFileVersion) -> Option<String> {
        match version {
            BlockFileVersion::Blugo => Some(String::from("https://block.energized.pro/bluGo/formats/domains.txt")),
            BlockFileVersion::Ultimate => Some(String::from("https://block.energized.pro/ultimate/formats/domains.txt")),
            BlockFileVersion::None => None
        }
    }

    fn get_file_name(version: BlockFileVersion) -> Option<String> {
        match version {
            BlockFileVersion::Blugo => Some(String::from("blugo.txt")),
            BlockFileVersion::Ultimate => Some(String::from("ultimate.txt")),
            BlockFileVersion::None => None
        }
    }


    pub fn from_download(version: BlockFileVersion, format: FilterFormat) -> Filter {
        Filter {
            format: format,
            domains_vector: None,
        }
    }

    pub fn from_disk(version: BlockFileVersion, format: FilterFormat) -> Result<Filter, std::io::Error> {
        match (Filter::get_file_name(version), format) {
            (None, format) => Ok(Filter { format: format, domains_vector: None }),
            (Some(file_name), FilterFormat::Vector) => {
                let file = File::open(file_name)?;
                let lines = io::BufReader::new(file)
                    .lines()
                    .filter_map(|maybe_line| {
                        match maybe_line {
                            Ok(line) => if line.starts_with("#") { None } else { Some(line) }
                            _ => None
                        }
                    })
                    .collect::<Vec<String>>();
                Ok(Filter {
                    format: FilterFormat::Vector,
                    domains_vector: Some(lines)
                })
            },
            _ => panic!()
        }
    }

    pub fn is_filtered(&self, name: String) -> bool {
        match self.format {
            FilterFormat::Vector => self.domains_vector.as_ref().unwrap().into_iter().any(|line| line == &name),
            _ => false,
        }
    }
}

use crate::cli::*;
use crate::filter_statistics::*;
use crate::tree::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

pub enum FilterVersion {
    None,
    Blu,
    Ultimate,
    Test,
}

#[derive(Clone)]
pub enum FilterFormat {
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
}

impl Filter {
    pub fn from_opt(opt: &Opt) -> Filter {
        let filter_version = match &opt.filter_list[..] {
            "none" => FilterVersion::None,
            "blu" => FilterVersion::Blu,
            "ultimate" => FilterVersion::Ultimate,
            "test" => FilterVersion::Test,
            _ => panic!("Filter list is not valid"),
        };
        let filter_format = if opt.small {
            FilterFormat::Vector
        } else {
            FilterFormat::Tree
        };
        let filters_path = opt.filters_path.clone().unwrap_or(PathBuf::from("./"));
        let allowed = opt.allowed.clone().unwrap_or(vec![]);

        Filter::from_disk(filter_version, filter_format, filters_path, allowed).expect("Couldn't load filter")
    }

    fn get_file_name(version: FilterVersion) -> Option<String> {
        match version {
            FilterVersion::Blu => Some(String::from("blu.txt")),
            FilterVersion::Ultimate => Some(String::from("ultimate.txt")),
            FilterVersion::Test => Some(String::from("test_filter.txt")),
            FilterVersion::None => None,
        }
    }

    pub fn from_disk(
        version: FilterVersion,
        format: FilterFormat,
        path: PathBuf,
        allowed: Vec<String>,
    ) -> Result<Filter, std::io::Error> {
        let lines = if let Some(file_name) = Filter::get_file_name(version) {
            let file = File::open(path.join(file_name))?;
            let mut vec = io::BufReader::new(file)
                .lines()
                .filter_map(|maybe_line| match maybe_line {
                    Ok(line) => {
                        if line.starts_with("#") {
                            None
                        } else if filtered_by(&line, |name| {
                            let result = allowed.binary_search(name).ok();
                            result.and_then(|i| allowed.get(i).map(|s| s.clone()))
                        })
                        .is_some()
                        {
                            None
                        } else {
                            Some(line)
                        }
                    }
                    _ => None,
                })
                .collect::<Vec<String>>();
            vec.sort();
            vec
        } else {
            vec![]
        };

        match format {
            FilterFormat::Vector => Ok(Filter {
                format: format,
                vector: Some(lines),
                hash: None,
                tree: None,
                statistics: FilterStatistics::new(),
            }),
            FilterFormat::Hash => {
                let mut hash = HashSet::new();
                for line in lines {
                    hash.insert(line);
                }
                Ok(Filter {
                    format: format,
                    vector: None,
                    hash: Some(hash),
                    tree: None,
                    statistics: FilterStatistics::new(),
                })
            }
            FilterFormat::Tree => {
                let mut tree = Tree::new();
                for line in lines {
                    tree.insert(&line);
                }
                Ok(Filter {
                    format: format,
                    vector: None,
                    hash: None,
                    tree: Some(tree),
                    statistics: FilterStatistics::new(),
                })
            }
        }
    }

    pub fn filtered_by(&mut self, name: &String) -> Option<String> {
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
        let name = parts.get(i..parts.len()).unwrap().join(".");
        contains(&name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_all_types() {
        vec![FilterFormat::Vector, FilterFormat::Hash, FilterFormat::Tree]
            .iter()
            .for_each(move |format| {
                let mut filter = Filter::from_disk(FilterVersion::Test, format.clone(), PathBuf::from("./"), vec![])
                    .expect("Couldn't load filter");

                assert_eq!(
                    Some(String::from("imateapot.org")),
                    filter.filtered_by(&String::from("imateapot.org"))
                );
                assert_eq!(
                    Some(String::from("imateapot.org")),
                    filter.filtered_by(&String::from("www.imateapot.org"))
                );
                assert_eq!(
                    Some(String::from("imateapot.org")),
                    filter.filtered_by(&String::from("m.www.imateapot.org"))
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.ca")));
                assert_eq!(
                    Some(String::from("www.imateapot.info")),
                    filter.filtered_by(&String::from("www.imateapot.info"))
                );
                assert_eq!(
                    Some(String::from("www.imateapot.info")),
                    filter.filtered_by(&String::from("m.www.imateapot.info"))
                );
                assert_eq!(None, filter.filtered_by(&String::from("imateapot.info")));
                assert_eq!(None, filter.filtered_by(&String::from("org")));
                assert_eq!(None, filter.filtered_by(&String::from("com")));
            });
    }

    #[test]
    fn allowed() {
        vec![FilterFormat::Vector, FilterFormat::Hash, FilterFormat::Tree]
            .iter()
            .for_each(move |format| {
                let mut filter = Filter::from_disk(
                    FilterVersion::Test,
                    format.clone(),
                    PathBuf::from("./"),
                    vec![String::from("imateapot.org")],
                )
                .expect("Couldn't load filter");

                assert_eq!(None, filter.filtered_by(&String::from("imateapot.org")));
                assert_eq!(None, filter.filtered_by(&String::from("www.imateapot.org")));
            });
    }
}

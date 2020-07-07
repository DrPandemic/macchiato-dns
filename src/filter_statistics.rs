use lru::LruCache;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use smartstring::{LazyCompact, SmartString};
use std::collections::HashMap;

type CompactString = SmartString<LazyCompact>;

pub struct FilterStatistics {
    pub data: LruCache<CompactString, u32>,
}

#[derive(serde::Serialize)]
pub struct SerializableFilterStatistics {
    pub data: HashMap<String, u32>,
}

impl SerializableFilterStatistics {
    fn from_filter_statistics(statistics: &FilterStatistics) -> SerializableFilterStatistics {
        let mut data: HashMap<String, u32> = HashMap::new();
        for (k, v) in statistics.data.iter() {
            data.insert(k.clone().into(), v.clone());
        }
        SerializableFilterStatistics { data: data }
    }
}

impl FilterStatistics {
    pub fn new() -> FilterStatistics {
        FilterStatistics {
            data: LruCache::new(500),
        }
    }

    pub fn increment(&mut self, name: &CompactString) {
        let count = match self.data.get(name) {
            Some(count) => count + 1,
            None => 1,
        };
        self.data.put(name.clone().into(), count);
    }
}

impl Serialize for FilterStatistics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("FilterStatistics", 3)?;
        state.serialize_field("capacity", &self.data.cap())?;
        state.serialize_field("length", &self.data.len())?;
        state.serialize_field("data", &SerializableFilterStatistics::from_filter_statistics(self))?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let mut stats = FilterStatistics::new();
        stats.increment(&String::from("imateapot.org").into());
        stats.increment(&String::from("imateapot.info").into());
        stats.increment(&String::from("imateapot.org").into());
        assert_eq!(Some(&2u32), stats.data.get(&CompactString::from("imateapot.org")));
        assert_eq!(Some(&1u32), stats.data.get(&CompactString::from("imateapot.info")));
    }
}

use lru::LruCache;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use smartstring::{LazyCompact, SmartString};
use std::{collections::HashMap, num::NonZeroUsize, time::SystemTime};

type CompactString = SmartString<LazyCompact>;

pub struct FilterStatistics {
    pub data: LruCache<CompactString, (u32, SystemTime)>,
}

#[derive(serde::Serialize)]
pub struct SerializableFilterStatistics {
    pub data: HashMap<String, (u32, SystemTime)>,
}

impl SerializableFilterStatistics {
    fn from_filter_statistics(statistics: &FilterStatistics) -> SerializableFilterStatistics {
        let mut data: HashMap<String, (u32, SystemTime)> = HashMap::new();
        for (k, v) in statistics.data.iter() {
            data.insert(k.clone().into(), v.clone());
        }
        SerializableFilterStatistics { data }
    }
}

impl FilterStatistics {
    pub fn new() -> FilterStatistics {
        FilterStatistics {
            data: LruCache::new(NonZeroUsize::new(500).unwrap()),
        }
    }

    pub fn increment(&mut self, name: &CompactString) {
        let count = match self.data.get(name) {
            Some((count, _)) => count + 1,
            None => 1,
        };
        self.data.put(name.clone().into(), (count, SystemTime::now()));
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
        let (a, _) = stats.data.get(&CompactString::from("imateapot.org")).unwrap();
        assert_eq!(&2u32, a);
        let (b, _) = stats.data.get(&CompactString::from("imateapot.info")).unwrap();
        assert_eq!(&1u32, b);
    }
}

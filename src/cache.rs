use crate::message::*;

use core::result::Result;
use std::num::NonZeroUsize;
use lru::LruCache;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Cache {
    pub data: LruCache<(String, u16), (SystemTime, Message)>,
}

#[derive(serde::Serialize)]
pub struct SerializableCacheEntry {
    pub valid_until: u64,
    pub message: Message,
}

#[derive(serde::Serialize)]
pub struct SerializableCache {
    pub data: Vec<SerializableCacheEntry>,
}

impl SerializableCache {
    fn from_cache(cache: &Cache) -> SerializableCache {
        let mut data: Vec<SerializableCacheEntry> = vec![];
        for (_, v) in cache.data.iter() {
            data.push(SerializableCacheEntry {
                valid_until: v.0.duration_since(UNIX_EPOCH).expect("Time before EPOCH").as_secs(),
                message: v.1.clone(),
            });
        }
        SerializableCache { data: data }
    }
}

impl Serialize for Cache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Cache", 3)?;
        state.serialize_field("capacity", &self.data.cap())?;
        state.serialize_field("length", &self.data.len())?;
        state.serialize_field("data", &SerializableCache::from_cache(&self))?;
        state.end()
    }
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            data: LruCache::new(NonZeroUsize::new(500).unwrap()),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(&mut self, query: &Message) -> Option<(Message, Duration)> {
        let question = query.question().ok()?;
        let key = (question.qname().ok()?.join("."), question.qtype().ok()?);
        let (time, message) = self.data.get(&key)?;
        let now = SystemTime::now();
        let response_ttl = time.clone();
        if now.cmp(&response_ttl) == Ordering::Less {
            let mut cached = (*message).clone();
            cached.set_id(query.id().ok()?).ok()?;
            let diff = response_ttl.duration_since(now).ok()?;
            if cached.set_response_ttl(diff.as_secs() as u32).is_ok() {
                Some((cached, diff))
            } else {
                None
            }
        } else {
            self.data.pop(&key);
            None
        }
    }

    pub fn put(&mut self, message: &Message) -> Option<()> {
        if let (Ok((responses, _, _)), Ok(question)) = (message.resource_records(), message.question()) {
            if responses.len() > 0 {
                let ttl = SystemTime::now()
                    .checked_add(Duration::from_secs(responses[0].ttl as u64))
                    .unwrap_or(SystemTime::now());
                if ttl > SystemTime::now() {
                    // I think this is wrong. What if the TTLs are different?
                    let key = (responses[0].name.join("."), question.qtype().ok()?);
                    self.data.put(key, (ttl, message.clone()));
                }
            }
        }

        Some(())
    }

    pub fn remove(&mut self, name: &String) {
        let mut keys_to_delete = vec![];
        for (key, _) in self.data.iter() {
            let key_name = &key.0;
            if name == key_name {
                keys_to_delete.push(key.clone());
            }
        }

        for key in keys_to_delete {
            self.data.pop(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    const IMATEAPOT_QUESTION: [u8; 46] = [
        57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97, 112, 111, 116, 3, 111,
        114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0,
    ];

    const IMATEAPOT_ANSWER: [u8; 95] = [
        57, 32, 129, 128, 0, 1, 0, 2, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97, 112, 111, 116, 3,
        111, 114, 103, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 84, 64, 0, 21, 5, 115, 104, 111, 112, 115, 9, 109,
        121, 115, 104, 111, 112, 105, 102, 121, 3, 99, 111, 109, 0, 192, 47, 0, 1, 0, 1, 0, 0, 5, 23, 0, 4, 23, 227,
        38, 64, 0, 0, 41, 2, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn test_can_retrieve() {
        let question = parse_message(IMATEAPOT_QUESTION.to_vec());
        let answer = parse_message(IMATEAPOT_ANSWER.to_vec());
        let mut cache = Cache::new();
        cache.put(&answer).unwrap();
        thread::sleep(time::Duration::from_secs(1));
        let (cached, time_left) = cache.get(&question).unwrap();
        assert_eq!(question.id().unwrap(), cached.id().unwrap());

        let in_ttl = answer.resource_records().unwrap().0[0].ttl;
        let out_ttl = cached.resource_records().unwrap().0[0].ttl;

        assert!(in_ttl > out_ttl);
        assert!(time_left.as_millis() > 500);
    }
}

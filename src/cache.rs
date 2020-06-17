use crate::message::*;
use lru::LruCache;
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

pub struct Cache {
    pub data: LruCache<(String, u16), (SystemTime, Message)>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            data: LruCache::new(500),
        }
    }

    pub fn get(&mut self, query: &Message) -> Option<Message> {
        let question = query.question().ok()?;
        let key = (question.qname().ok()?.join("."), question.qtype().ok()?);
        let (time, message) = self.data.get(&key)?;
        let now = SystemTime::now();
        let response_ttl = time.clone();
        if now.cmp(&response_ttl) == Ordering::Less {
            let mut cached = (*message).clone();
            cached.set_id(query.id().ok()?).ok()?;
            if cached
                .set_response_ttl(response_ttl.duration_since(now).ok()?.as_secs() as u32)
                .is_ok()
            {
                Some(cached)
            } else {
                None
            }
        } else {
            self.data.pop(&key);
            None
        }
    }

    pub fn put(&mut self, message: &Message) -> Option<()> {
        if let (Ok((responses, _, _)), Ok(question)) =
            (message.resource_records(), message.question())
        {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    const IMATEAPOT_QUESTION: [u8; 46] = [
        57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97,
        112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0,
    ];

    const IMATEAPOT_ANSWER: [u8; 95] = [
        57, 32, 129, 128, 0, 1, 0, 2, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97,
        112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 84, 64, 0, 21,
        5, 115, 104, 111, 112, 115, 9, 109, 121, 115, 104, 111, 112, 105, 102, 121, 3, 99, 111,
        109, 0, 192, 47, 0, 1, 0, 1, 0, 0, 5, 23, 0, 4, 23, 227, 38, 64, 0, 0, 41, 2, 0, 0, 0, 0,
        0, 0, 0,
    ];

    #[test]
    fn test_can_retrieve() {
        let question = parse_message(IMATEAPOT_QUESTION.to_vec());
        let answer = parse_message(IMATEAPOT_ANSWER.to_vec());
        let mut cache = Cache::new();
        cache.put(&answer).unwrap();
        thread::sleep(time::Duration::from_secs(1));
        let cached = cache.get(&question).unwrap();
        assert_eq!(question.id().unwrap(), cached.id().unwrap());

        let in_ttl = answer.resource_records().unwrap().0[0].ttl;
        let out_ttl = cached.resource_records().unwrap().0[0].ttl;

        assert!(in_ttl > out_ttl);
    }
}

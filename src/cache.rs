use crate::message::*;
use lru::LruCache;
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
        let question = query.question()?;
        let key = (question.qname()?.join("."), question.qtype()?);
        let (time, message) = self.data.get(&key)?;
        let time_difference = SystemTime::now().duration_since(time.clone()).ok()?;
        if time > &SystemTime::now() {
            let mut response = (*message).clone();
            response.set_id(query.id()?)?;
            response.set_response_ttl(time_difference.as_secs() as u32);
            Some(response)
        } else {
            self.data.pop(&key);
            None
        }
    }

    pub fn put(&mut self, message: &Message) -> Option<()> {
        if let (Some((responses, _, _)), Some(question)) =
            (message.resource_records(), message.question())
        {
            if responses.len() > 0 {
                let ttl = SystemTime::now()
                    .checked_add(Duration::from_secs(responses[0].ttl as u64))
                    .unwrap_or(SystemTime::now());
                if ttl > SystemTime::now() {
                    // I think this is wrong. What if the TTLs are different?
                    self.data.put(
                        (responses[0].name.join("."), question.qtype()?),
                        (ttl, message.clone()),
                    );
                }
            }
        }

        Some(())
    }
}

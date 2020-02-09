use crate::message::*;
use lru::LruCache;
use std::time::{Duration, SystemTime};

pub struct Cache {
    pub data: LruCache<String, (SystemTime, Message)>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            data: LruCache::new(500),
        }
    }

    pub fn get(&mut self, query: &Message) -> Option<Message> {
        if let Some(question) = query.question() {
            let name = question.qname().join(".");
            let (time, message) = self.data.get(&name)?;
            if time > &SystemTime::now() {
                let mut response = (*message).clone();
                response.set_id(query.id());
                // if let Some(mut rrs) = response.resource_records() {
                //     if rrs.0.len() > 0 {
                //         rrs.0[0].set_response_ttl(1);
                //     }
                // }
                Some(response)
            } else {
                self.data.pop(&name);
                None
            }
        } else {
            None
        }
    }

    pub fn put(&mut self, message: &Message) {
        if let Some((responses, _, _)) = message.resource_records() {
            if responses.len() > 0 {
                let ttl = SystemTime::now()
                    .checked_add(Duration::from_secs(responses[0].ttl as u64))
                    .unwrap_or(SystemTime::now());
                if ttl > SystemTime::now() {
                    // I think this is wrong. What if the TTLs are different?
                    self.data
                        .put(responses[0].name.join("."), (ttl, message.clone()));
                }
            }
        }
    }
}

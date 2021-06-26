use std::collections::HashMap;

use crate::resolver_manager::ResolverManager;
use crate::ring_buffer::*;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Clone, serde::Serialize)]
pub struct Instrumentation {
    pub resolver: Option<String>,
    pub initial: SystemTime,
    pub request_sent: Option<SystemTime>,
    pub request_received: Option<SystemTime>,
}

#[derive(Clone, serde::Serialize)]
pub struct InstrumentationLog {
    pub data: RingBuffer<Instrumentation>,
}

impl Instrumentation {
    pub fn new() -> Instrumentation {
        Instrumentation {
            resolver: None,
            initial: SystemTime::now(),
            request_sent: None,
            request_received: None,
        }
    }

    pub fn set_request_sent(&mut self, resolver: String) {
        self.request_sent = Some(SystemTime::now());
        self.resolver = Some(resolver);
    }

    pub fn set_request_received(&mut self) {
        self.request_received = Some(SystemTime::now());
    }

    pub fn remote_timing(&self) -> Duration {
        if let (Some(a), Some(b)) = (self.request_sent, self.request_received) {
            b.duration_since(a).unwrap_or(Duration::new(0, 0))
        } else {
            Duration::new(0, 0)
        }
    }

    pub fn display(&self) {
        let total = SystemTime::now()
            .duration_since(self.initial)
            .unwrap_or(Duration::new(0, 0));
        println!(
            "{:?} in this server with a total of {:?}",
            total - self.remote_timing(),
            total
        );
    }
}

impl InstrumentationLog {
    pub fn new() -> InstrumentationLog {
        InstrumentationLog {
            data: RingBuffer::new(100),
        }
    }

    pub fn push(&mut self, instrumentation: Instrumentation) {
        self.data.push(instrumentation);
    }

    pub fn update_resolver_manager(&self, resolver_manager: Arc<Mutex<ResolverManager>>) {
        let mut resolver_manager = resolver_manager.lock().unwrap();
        let mut groups: HashMap<String, Vec<Duration>> = HashMap::new();
        for instrumentation in (&self.data).into_iter() {
            if let Some(resolver) = instrumentation.resolver.clone() {
                if !groups.contains_key(&resolver) {
                    groups.insert(resolver.clone(), vec![]);
                }

                groups.get_mut(&resolver).unwrap().push(instrumentation.remote_timing());
            }
        }
        for (key, group) in groups {
            let sum = group.iter().sum::<Duration>();
            resolver_manager.update_resolver(key, sum / group.len() as u32);
        }
    }
}

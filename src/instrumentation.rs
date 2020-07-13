use crate::ring_buffer::*;
use std::time::{Duration, SystemTime};

#[derive(Clone, serde::Serialize)]
pub struct Instrumentation {
    pub resolver: Option<String>,
    pub initial: SystemTime,
    pub request_sent: Option<SystemTime>,
    pub request_received: Option<SystemTime>,
}

#[derive(serde::Serialize)]
pub struct InstrumentationLog {
    data: RingBuffer<Instrumentation>,
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

    pub fn display(&self) {
        let remote_timing = if let (Some(a), Some(b)) = (self.request_sent, self.request_received) {
            b.duration_since(a).unwrap_or(Duration::new(0, 0))
        } else {
            Duration::new(0, 0)
        };

        let total = SystemTime::now()
            .duration_since(self.initial)
            .unwrap_or(Duration::new(0, 0));
        println!("{:?} in this server with a total of {:?}", total - remote_timing, total,);
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
}

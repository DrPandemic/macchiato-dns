use std::time::{Duration, SystemTime};

pub struct Instrumentation {
    pub initial: SystemTime,
    pub request_sent: Option<SystemTime>,
    pub request_received: Option<SystemTime>,
}

impl Instrumentation {
    pub fn new() -> Instrumentation {
        Instrumentation {
            initial: SystemTime::now(),
            request_sent: None,
            request_received: None,
        }
    }

    pub fn set_request_sent(&mut self) {
        self.request_sent = Some(SystemTime::now());
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
        println!(
            "{:?} in this server with a total of {:?}",
            total - remote_timing,
            total,
        );
    }
}

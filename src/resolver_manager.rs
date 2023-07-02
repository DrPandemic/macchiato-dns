use rand::Rng;
use std::time::Duration;

const RANDOM_FACTOR: f32 = 0.05;

pub struct ResolverManager {
    resolvers: Vec<(String, Option<(&'static str, &'static str)>, Duration)>,
}

impl Default for ResolverManager {
    fn default() -> ResolverManager {
        ResolverManager {
            resolvers: vec![
                (
                    "https://8.8.8.8/dns-query".to_string(),
                    Some(("authority", "dns.google")),
                    Duration::new(0, 0),
                ),
                ("https://1.1.1.1/dns-query".to_string(), None, Duration::new(0, 0)),
                ("https://9.9.9.9/dns-query".to_string(), None, Duration::new(0, 0)),
            ],
        }
    }
}

impl ResolverManager {
    pub fn get_resolver(&mut self) -> (String, Option<(&'static str, &'static str)>) {
        let mut rng = rand::thread_rng();

        let entry = if rng.gen::<f32>() < RANDOM_FACTOR {
            &self.resolvers[rng.gen_range(0..self.resolvers.len())]
        } else {
            self.resolvers.iter().min_by_key(|e| e.2).unwrap()
        };

        (entry.0.clone(), entry.1)
    }

    pub fn update_resolver(&mut self, resolver: String, average: Duration) {
        for entry in self.resolvers.iter_mut() {
            if entry.0 == resolver {
                entry.2 = average;
                break;
            }
        }
    }
}

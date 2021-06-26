use crate::web::AppState;
use crate::instrumentation::*;

use actix_web::{get, web, HttpResponse};
use std::collections::{HashMap, HashSet};

#[get("/prometheus_metrics")]
pub async fn metrics(data: web::Data<AppState>) -> HttpResponse {
    let instrumentation_log = data.instrumentation_log.lock().unwrap();

    let result = format!("
# HELP macchiato_resolver_latency Average time it takes for the DNS server to return an answer.
# TYPE macchiato_resolver_latency gauge
{}
", resolver_average(&(*instrumentation_log)));

    HttpResponse::Ok()
        .content_type("plain/text")
        .body(result)
}

fn resolver_average(log: &InstrumentationLog) -> String {
    let mut durations: HashMap<String, Vec<u128>> = HashMap::new();
    let resolvers: HashSet<String> = (&log.data).into_iter()
        .map(|entry| entry.resolver.map_or(String::from(""), |entry| entry))
        .filter(|entry| entry != "")
        .collect();
    for resolver in resolvers {
        durations.insert(resolver, vec![]);
    }

    for entry in &log.data {
        if let Some(ref resolver) = entry.resolver {
            let duration = entry.remote_timing();
            durations.get_mut(&resolver.clone()).expect("There was an error building the result map.").push(duration.as_millis());
        }
    }

    let results: Vec<String> = durations.iter()
        .map(|(k, v)| {
            let average = v.iter().sum::<u128>() / v.len() as u128;
            format!("macchiato_resolver_latency{{resolver=\"{}\"}} {}", k, average)
        }).collect();

    String::from(results.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[test]
    fn test_resolver_average() {
        let mut log = InstrumentationLog::new();
        let mut inst0 = Instrumentation::new();
        let mut inst1 = Instrumentation::new();
        let mut inst2 = Instrumentation::new();
        inst0.set_request_sent(String::from("8.8.8.8"));
        inst1.set_request_sent(String::from("1.1.1.1"));
        inst2.set_request_sent(String::from("8.8.8.8"));
        thread::sleep(time::Duration::from_secs(1));
        inst0.set_request_received();
        inst1.set_request_received();
        thread::sleep(time::Duration::from_secs(1));
        inst2.set_request_received();
        log.push(inst0);
        log.push(inst1);
        log.push(inst2);

        assert!(resolver_average(&log).contains(&String::from("macchiato_resolver_latency{resolver=\"8.8.8.8\"} 1500")));
        assert!(resolver_average(&log).contains(&String::from("macchiato_resolver_latency{resolver=\"1.1.1.1\"} 1000")));
    }
}

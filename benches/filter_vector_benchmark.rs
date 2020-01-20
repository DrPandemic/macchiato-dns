extern crate criterion;
extern crate dns;

use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dns::filter::*;


pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");
    group.measurement_time(Duration::new(5, 0));

    bench(&mut group, FilterFormat::Vector, "vector");
    bench(&mut group, FilterFormat::Hash, "hash");
    bench(&mut group, FilterFormat::Cuckoo, "cuckoo");
    group.finish();
}

fn bench(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime> , format: FilterFormat, name: &str) {
    let filter = Filter::from_disk(BlockFileVersion::Ultimate, format).expect("Couldn't load filter");
    group.bench_function(format!("not in {}", name), |b| {
        b.iter(|| {
            filter.is_filtered(black_box(String::from("notblacklisted.com")))
        })
    });
    group.bench_function(format!("beginning of {}", name), |b| {
        b.iter(|| {
            filter.is_filtered(black_box(String::from("0.015.openvpn.btcchina.com")))
        })
    });
    group.bench_function(format!("end of {}", name), |b| {
        b.iter(|| {
            filter.is_filtered(black_box(String::from("zzzzzz.com")))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

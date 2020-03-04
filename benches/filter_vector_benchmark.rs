extern crate criterion;
extern crate dns;
use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dns::filter::*;
use std::time::Duration;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");
    group.measurement_time(Duration::new(5, 0));

    bench(&mut group, FilterFormat::Vector, "vector");
    bench(&mut group, FilterFormat::Hash, "hash");
    bench(&mut group, FilterFormat::Tree, "tree");
    group.finish();
}

fn bench(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    format: FilterFormat,
    name: &str,
) {
    let filter = Filter::from_disk(FilterVersion::Ultimate, format, PathBuf::from("./"))
        .expect("Couldn't load filter");
    group.bench_function(format!("not in {}", name), |b| {
        b.iter(|| filter.is_filtered(black_box(&String::from("notblacklisted.com"))))
    });
    group.bench_function(format!("beginning of {}", name), |b| {
        b.iter(|| filter.is_filtered(black_box(&String::from("0.015.openvpn.btcchina.com"))))
    });
    group.bench_function(format!("end of {}", name), |b| {
        b.iter(|| filter.is_filtered(black_box(&String::from("zzzzzz.com"))))
    });
    group.bench_function(format!("long domain {}", name), |b| {
        b.iter(|| {
            filter.is_filtered(black_box(&String::from(
                "aaa.bbb.ccc.ddd.eee.fff.ggg.hhh.iii.jjj.com",
            )))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

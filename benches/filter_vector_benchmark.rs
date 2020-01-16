extern crate criterion;
extern crate dns;

use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dns::filter::*;


pub fn criterion_benchmark(c: &mut Criterion) {
    let filter_vector = Filter::from_disk(BlockFileVersion::Ultimate, FilterFormat::Vector).expect("Couldn't load filter");
    let filter_hash = Filter::from_disk(BlockFileVersion::Ultimate, FilterFormat::Hash).expect("Couldn't load filter");

    let mut group = c.benchmark_group("filter");
    group.measurement_time(Duration::new(17, 0));

    group.bench_function("not in vector", |b| {
        b.iter(|| {
            filter_vector.is_filtered(black_box(String::from("notblacklisted.com")))
        })
    });
    group.bench_function("beginning of vector", |b| {
        b.iter(|| {
            filter_vector.is_filtered(black_box(String::from("0.015.openvpn.btcchina.com")))
        })
    });
    group.bench_function("end of vector", |b| {
        b.iter(|| {
            filter_vector.is_filtered(black_box(String::from("zzzzzz.com")))
        })
    });
    group.bench_function("not in hash", |b| {
        b.iter(|| {
            filter_hash.is_filtered(black_box(String::from("notblacklisted.com")))
        })
    });
    group.bench_function("beginning of hash", |b| {
        b.iter(|| {
            filter_hash.is_filtered(black_box(String::from("0.015.openvpn.btcchina.com")))
        })
    });
    group.bench_function("end of hash", |b| {
        b.iter(|| {
            filter_hash.is_filtered(black_box(String::from("zzzzzz.com")))
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use criterion::{criterion_group, criterion_main, Criterion};
use pprof::validate;

fn bench_validate_addr(c: &mut Criterion) {
    c.bench_function("validate stack addr", |b| {
        let stack_addrs: [u128; 100] = [0; 100];

        b.iter(|| {
            stack_addrs.iter().for_each(|item| {
                validate(item);
            })
        })
    });

    c.bench_function("validate heap addr", |b| {
        let heap_addrs: Vec<u128> = vec![0; 100];

        b.iter(|| {
            heap_addrs.iter().for_each(|item| {
                validate(item);
            })
        })
    });
}

criterion_group!(benches, bench_validate_addr);
criterion_main!(benches);

use criterion::{criterion_group, criterion_main};

#[path = "api_candidates/hecs_dense.rs"]
mod hecs_dense;

criterion_group!(
    api_candidates,
    hecs_dense::bench_10k,
    hecs_dense::bench_100k,
    hecs_dense::bench_1m,
);
criterion_main!(api_candidates);

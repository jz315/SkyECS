use criterion::{criterion_group, criterion_main};

#[path = "api_candidates/fixed_sequence.rs"]
mod fixed_sequence;
#[path = "api_candidates/hecs_dense.rs"]
mod hecs_dense;
#[path = "api_candidates/sky_gameplay.rs"]
mod sky_gameplay;
#[path = "api_candidates/sky_heavy.rs"]
mod sky_heavy;

criterion_group!(
    api_candidates,
    fixed_sequence::run,
    hecs_dense::bench_10k,
    hecs_dense::bench_100k,
    hecs_dense::bench_1m,
    sky_gameplay::run,
    sky_heavy::run,
);
criterion_main!(api_candidates);

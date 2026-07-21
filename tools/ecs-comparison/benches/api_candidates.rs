use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs_comparison::sky;
use std::time::Duration;

#[path = "api_candidates/hecs_dense.rs"]
mod hecs_dense;

fn sky_gameplay_api_candidates(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sky_gameplay_api_candidates");
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    sky::bench_gameplay_api_candidates(&mut group);
    group.finish();
}

criterion_group!(
    api_candidates,
    sky_gameplay_api_candidates,
    hecs_dense::bench_10k,
    hecs_dense::bench_100k,
    hecs_dense::bench_1m,
);
criterion_main!(api_candidates);

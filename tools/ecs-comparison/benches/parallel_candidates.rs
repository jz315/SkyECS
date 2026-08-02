use criterion::{criterion_group, criterion_main};
use sky_ecs_comparison::parallel::bench_parallel_candidates;

criterion_group!(parallel_candidates, bench_parallel_candidates);
criterion_main!(parallel_candidates);

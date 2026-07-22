use criterion::{criterion_group, criterion_main};

#[path = "comparison/suite.rs"]
mod suite;

criterion_group!(
    comparison_benches,
    suite::bench_insert,
    suite::bench_native_bulk,
    suite::bench_iteration,
    suite::bench_iteration_large,
    suite::bench_iteration_1m,
    suite::bench_fragmented_iteration,
    suite::bench_random_fragmented_iteration,
    suite::bench_heavy_compute,
    suite::bench_entity_id_random_access,
    suite::bench_fixed_sequence_access,
    suite::bench_entity_ops,
    suite::bench_gameplay_frame,
);
criterion_main!(comparison_benches);

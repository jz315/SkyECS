use super::fixtures::{fresh_bulk_fixture, insert_bulk, reused_bulk_fixture};
use criterion::{BatchSize, Criterion};
use std::time::Duration;

pub(crate) fn bench_bulk_metadata(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("structural_writes/bulk_metadata_10k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40);
    group.bench_function("fresh_ids", |bencher| {
        bencher.iter_batched_ref(fresh_bulk_fixture, insert_bulk, BatchSize::LargeInput);
    });
    group.bench_function("reused_ids", |bencher| {
        bencher.iter_batched_ref(reused_bulk_fixture, insert_bulk, BatchSize::LargeInput);
    });
    group.finish();
}

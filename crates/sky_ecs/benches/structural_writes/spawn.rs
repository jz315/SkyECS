use super::fixtures::*;
use criterion::{BatchSize, Criterion};
use std::hint::black_box;
use std::time::Duration;

pub(crate) fn bench_spawn_and_despawn(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("structural_writes/spawn_1k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40);

    group.bench_function("fresh_ids_cold_storage", |bencher| {
        bencher.iter_batched_ref(
            fresh_world,
            |world| {
                spawn_light_rows(world);
                black_box(world);
            },
            BatchSize::LargeInput,
        );
    });
    group.bench_function("reused_ids_cold_storage", |bencher| {
        bencher.iter_batched_ref(
            cold_storage_with_reused_ids,
            |world| {
                spawn_light_rows(world);
                black_box(world);
            },
            BatchSize::LargeInput,
        );
    });
    group.bench_function("reused_ids_warm_storage", |bencher| {
        bencher.iter_batched_ref(
            warm_started_light_storage,
            |world| {
                spawn_light_rows(world);
                black_box(world);
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();

    let deletion_order = shuffled_deletion_order();
    let mut group = criterion.benchmark_group("structural_writes/despawn_1k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40);
    group.bench_function("random_swap_remove", |bencher| {
        bencher.iter_batched_ref(
            populated_light_world,
            |(world, entities)| {
                for &index in &deletion_order {
                    assert!(world.despawn(entities[index]));
                }
                black_box(world);
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();

    let mut fixture = ChurnFixture::new();
    let mut group = criterion.benchmark_group("structural_writes/churn_1k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40);
    group.bench_function("spawn_then_despawn", |bencher| {
        bencher.iter(|| {
            fixture.cycle();
            black_box(&fixture.world);
        });
    });
    group.finish();
}

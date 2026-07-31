use super::fixtures::ENTITY_COUNT;
use criterion::{BatchSize, Criterion};
use sky_ecs::World;
use std::hint::black_box;
use std::time::Duration;

#[derive(Clone, Copy)]
struct Anchor;

#[derive(Clone, Copy)]
struct C0(u64);

#[derive(Clone, Copy)]
struct C1(u64);

#[derive(Clone, Copy)]
struct C2(u64);

#[derive(Clone, Copy)]
struct C3(u64);

#[derive(Clone, Copy)]
struct Added(u64);

macro_rules! bench_copy_span_count {
    ($group:expr, $name:literal, $base:expr, $with_added:expr) => {{
        $group.bench_function(concat!($name, "/add_only"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::new();
                    let entities = (0..ENTITY_COUNT)
                        .map(|_| world.spawn($base))
                        .collect::<Vec<_>>();
                    (world, entities)
                },
                |(world, entities)| {
                    for &entity in entities.iter() {
                        assert!(world.insert(entity, Added(7)));
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
        $group.bench_function(concat!($name, "/remove_only"), |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::new();
                    let entities = (0..ENTITY_COUNT)
                        .map(|_| world.spawn($with_added))
                        .collect::<Vec<_>>();
                    (world, entities)
                },
                |(world, entities)| {
                    for &entity in entities.iter() {
                        assert!(world.remove::<Added>(entity));
                    }
                    black_box(world);
                },
                BatchSize::LargeInput,
            );
        });
    }};
}

pub(crate) fn bench_migration_copy_spans(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("structural_writes/migration_1k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40);

    bench_copy_span_count!(group, "copy_spans_0", (Anchor,), (Anchor, Added(7)));
    bench_copy_span_count!(group, "copy_spans_1", (C0(0),), (C0(0), Added(7)));
    bench_copy_span_count!(
        group,
        "copy_spans_2",
        (C0(0), C1(1)),
        (C0(0), C1(1), Added(7))
    );
    bench_copy_span_count!(
        group,
        "copy_spans_3",
        (C0(0), C1(1), C2(2)),
        (C0(0), C1(1), C2(2), Added(7))
    );
    bench_copy_span_count!(
        group,
        "copy_spans_4",
        (C0(0), C1(1), C2(2), C3(3)),
        (C0(0), C1(1), C2(2), C3(3), Added(7))
    );
    group.finish();
}

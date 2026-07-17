use super::*;

pub(super) fn prepared_insert_world() -> World {
    let mut world = World::new();
    // Register bundle/component metadata without creating workload entities or
    // reserving storage for the measured rows.
    let _ = world.register_bundle::<SuiteBundle>();
    world
}
pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_insert_10k/bevy", |b| {
        let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                world.spawn_batch(bundles.iter().copied());
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/bevy", |b| {
        let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                for &bundle in &bundles {
                    world.spawn(bundle);
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

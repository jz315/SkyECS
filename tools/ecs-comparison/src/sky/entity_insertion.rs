use super::*;

pub(super) fn prepared_insert_world() -> World {
    // Sky caches bundle metadata process-wide. Make that reusable schema work
    // explicit in setup while leaving the World empty and without row capacity.
    let _ = <SuiteBundle as Bundle>::archetype();
    World::new()
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_insert_10k/sky", |b| {
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

    group.bench_function("single_insert_10k/sky", |b| {
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

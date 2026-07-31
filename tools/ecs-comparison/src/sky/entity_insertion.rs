use super::*;

pub(super) fn prepared_insert_world() -> World {
    // Sky caches bundle metadata process-wide. Make that reusable schema work
    // explicit in setup while leaving the World empty and without row capacity.
    let _ = <SuiteBundle as Bundle>::archetype();
    World::new()
}

pub(super) struct BulkConstructionContext {
    pub world: World,
    pub columns: SuiteColumns,
}

pub(super) fn bulk_construction_context(columns: SuiteColumns) -> BulkConstructionContext {
    BulkConstructionContext {
        world: prepared_insert_world(),
        columns,
    }
}

pub(super) fn insert_bulk_from_columns(context: &mut BulkConstructionContext) {
    context
        .world
        .spawn_columns(&mut context.columns)
        .expect("suite columns have equal lengths");
}

pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/sky", |b| {
        b.iter_batched_ref(
            || bulk_construction_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_bulk_from_columns(context);
                black_box(&context.world);
            },
            construction_batch_size(),
        );
    });
}

pub fn bench_single_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
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
            construction_batch_size(),
        );
    });
}

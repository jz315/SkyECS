use super::*;

pub(super) fn prepared_insert_world() -> World {
    let mut world = World::new();
    // Register bundle/component metadata without creating workload entities or
    // reserving storage for the measured rows.
    let _ = world.register_bundle::<SuiteBundle>();
    world
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
    let BulkConstructionContext { world, columns } = context;
    drop(world.spawn_batch(drain_suite_columns(columns)));
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/bevy", |b| {
        b.iter_batched_ref(
            || bulk_construction_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_bulk_from_columns(context);
                black_box(&context.world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn bench_single_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
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

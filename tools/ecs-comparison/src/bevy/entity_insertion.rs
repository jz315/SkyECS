use super::*;

pub(super) fn prepared_insert_world() -> World {
    let mut world = World::new();
    // Register bundle/component metadata without creating workload entities or
    // reserving storage for the measured rows.
    let _ = world.register_bundle::<SuiteBundle>();
    world
}

pub(super) struct NativeBulkContext {
    pub world: World,
    pub bundles: Option<Vec<SuiteBundle>>,
}

pub(super) fn native_bulk_context(columns: SuiteColumns) -> NativeBulkContext {
    NativeBulkContext {
        world: prepared_insert_world(),
        bundles: Some(suite_columns_into_bundles(columns)),
    }
}

pub(super) fn insert_native_bulk(context: &mut NativeBulkContext) {
    let bundles = context
        .bundles
        .take()
        .expect("native bundle batch is consumed once");
    drop(context.world.spawn_batch(bundles));
}
pub fn bench_native_bulk(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("insert_10k/bevy", |b| {
        b.iter_batched_ref(
            || native_bulk_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_native_bulk(context);
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

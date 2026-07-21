use super::*;

pub(super) fn prepared_insert_world() -> World {
    World::new()
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
    drop(context.world.bulk_add_entity(bundles));
}
pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("native_bulk_insert_10k/shipyard", |b| {
        b.iter_batched_ref(
            || native_bulk_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_native_bulk(context);
                black_box(&context.world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/shipyard", |b| {
        let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                for &bundle in &bundles {
                    world.add_entity(bundle);
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

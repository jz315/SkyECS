use super::*;

pub(super) fn prepared_insert_world() -> World {
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
    let BulkConstructionContext { world, columns } = context;
    // Shipyard inserts the entities eagerly; the returned iterator only exposes
    // the IDs of entities that already exist in the world.
    let _new_entity_ids = world.bulk_add_entity(drain_suite_columns(columns));
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/shipyard", |b| {
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
            construction_batch_size(),
        );
    });
}

use super::*;
use hecs::{ColumnBatch, ColumnBatchType};

pub(super) fn prepared_insert_world() -> World {
    World::new()
}

pub(super) struct BulkConstructionContext {
    pub world: World,
    pub columns: SuiteColumns,
}

pub(super) fn build_column_batch(columns: &mut SuiteColumns) -> ColumnBatch {
    let (transforms, positions, rotations, velocities) = columns;
    let count = transforms.len();
    assert_eq!(positions.len(), count);
    assert_eq!(rotations.len(), count);
    assert_eq!(velocities.len(), count);
    let mut batch_type = ColumnBatchType::new();
    batch_type
        .add::<TransformComponent>()
        .add::<PositionComponent>()
        .add::<RotationComponent>()
        .add::<VelocityComponent>();
    let builder = batch_type.into_batch(count as u32);
    {
        let mut writer = builder.writer::<TransformComponent>().unwrap();
        for value in transforms.drain(..) {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<PositionComponent>().unwrap();
        for value in positions.drain(..) {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<RotationComponent>().unwrap();
        for value in rotations.drain(..) {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<VelocityComponent>().unwrap();
        for value in velocities.drain(..) {
            assert!(writer.push(value).is_ok());
        }
    }
    builder
        .build()
        .expect("all native batch columns are complete")
}

pub(super) fn bulk_construction_context(columns: SuiteColumns) -> BulkConstructionContext {
    BulkConstructionContext {
        world: prepared_insert_world(),
        columns,
    }
}

pub(super) fn insert_bulk_from_columns(context: &mut BulkConstructionContext) {
    let batch = build_column_batch(&mut context.columns);
    drop(context.world.spawn_column_batch(batch));
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/hecs", |b| {
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
    group.bench_function("single_insert_10k/hecs", |b| {
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

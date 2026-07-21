use super::*;
use hecs::{ColumnBatch, ColumnBatchType};

pub(super) fn prepared_insert_world() -> World {
    World::new()
}

pub(super) struct NativeBulkContext {
    pub world: World,
    pub batch: Option<ColumnBatch>,
}

pub(super) fn build_column_batch(columns: SuiteColumns) -> ColumnBatch {
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
        for value in transforms {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<PositionComponent>().unwrap();
        for value in positions {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<RotationComponent>().unwrap();
        for value in rotations {
            assert!(writer.push(value).is_ok());
        }
    }
    {
        let mut writer = builder.writer::<VelocityComponent>().unwrap();
        for value in velocities {
            assert!(writer.push(value).is_ok());
        }
    }
    builder
        .build()
        .expect("all native batch columns are complete")
}

pub(super) fn native_bulk_context(columns: SuiteColumns) -> NativeBulkContext {
    NativeBulkContext {
        world: prepared_insert_world(),
        batch: Some(build_column_batch(columns)),
    }
}

pub(super) fn insert_native_bulk(context: &mut NativeBulkContext) {
    let batch = context.batch.take().expect("native batch is consumed once");
    drop(context.world.spawn_column_batch(batch));
}
pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("native_bulk_insert_10k/hecs", |b| {
        b.iter_batched_ref(
            || native_bulk_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_native_bulk(context);
                black_box(&context.world);
            },
            BatchSize::SmallInput,
        );
    });

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

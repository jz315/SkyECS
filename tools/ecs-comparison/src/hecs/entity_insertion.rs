use super::*;
use hecs::{ColumnBatch, ColumnBatchType};

pub(super) fn prepared_insert_world() -> World {
    World::new()
}

fn column_batch_type() -> ColumnBatchType {
    let mut batch_type = ColumnBatchType::new();
    batch_type
        .add::<TransformComponent>()
        .add::<PositionComponent>()
        .add::<RotationComponent>()
        .add::<VelocityComponent>();
    batch_type
}

pub(super) struct BulkConstructionContext {
    pub world: World,
    pub columns: SuiteColumns,
    batch_type: Option<ColumnBatchType>,
}

fn build_column_batch(columns: &mut SuiteColumns, batch_type: ColumnBatchType) -> ColumnBatch {
    let (transforms, positions, rotations, velocities) = columns;
    let count = transforms.len();
    assert_eq!(positions.len(), count);
    assert_eq!(rotations.len(), count);
    assert_eq!(velocities.len(), count);
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
        batch_type: Some(column_batch_type()),
    }
}

pub(super) fn insert_bulk_from_columns(context: &mut BulkConstructionContext) {
    let batch_type = context
        .batch_type
        .take()
        .expect("bulk construction context is single-use");
    let batch = build_column_batch(&mut context.columns, batch_type);
    drop(context.world.spawn_column_batch(batch));
}

#[cfg(feature = "api-experiments")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BulkSchemaCandidate {
    RebuildInTimedPath,
    PreparedInSetup,
}

#[cfg(feature = "api-experiments")]
pub fn measure_bulk_schema_candidate(candidate: BulkSchemaCandidate) -> std::time::Duration {
    let mut context = BulkConstructionContext {
        world: prepared_insert_world(),
        columns: suite_columns(SIMPLE_ENTITY_COUNT),
        batch_type: match candidate {
            BulkSchemaCandidate::RebuildInTimedPath => None,
            BulkSchemaCandidate::PreparedInSetup => Some(column_batch_type()),
        },
    };

    let start = std::time::Instant::now();
    let batch_type = context.batch_type.take().unwrap_or_else(column_batch_type);
    let batch = build_column_batch(&mut context.columns, batch_type);
    drop(context.world.spawn_column_batch(batch));
    let elapsed = start.elapsed();
    black_box(&context.world);
    elapsed
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/hecs", |b| {
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
            construction_batch_size(),
        );
    });
}

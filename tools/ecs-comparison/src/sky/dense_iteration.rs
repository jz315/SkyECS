use super::*;

pub(super) fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}

#[inline(never)]
fn move_chunk(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/sky", |b| {
        let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        assert_eq!(query.count(&world), SIMPLE_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk_fn(&mut world, move_chunk);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/sky", |b| {
        let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        assert_eq!(query.count(&world), LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk_fn(&mut world, move_chunk);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/sky", |b| {
        let mut world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        assert_eq!(query.count(&world), VERY_LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk_fn(&mut world, move_chunk);
            black_box(&world);
        });
    });
}

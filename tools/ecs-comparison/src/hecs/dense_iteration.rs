use super::*;

pub(super) fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}

pub(super) fn assert_prepared_count<Q: HecsQuery>(
    query: &mut PreparedQuery<Q>,
    world: &World,
    expected: usize,
) {
    assert_eq!(query.query(world).iter().count(), expected);
}
pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/hecs", |b| {
        let world = world_with_entities(SIMPLE_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        assert_prepared_count(&mut query, &world, SIMPLE_ENTITY_COUNT);
        b.iter(|| {
            for (pos, vel) in query.query(&world).iter() {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/hecs", |b| {
        let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        assert_prepared_count(&mut query, &world, LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            for (pos, vel) in query.query(&world).iter() {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/hecs", |b| {
        let world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        assert_prepared_count(&mut query, &world, VERY_LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            for (pos, vel) in query.query(&world).iter() {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

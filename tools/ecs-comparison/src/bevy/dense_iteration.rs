use super::*;

pub(super) fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}
pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/bevy", |b| {
        let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
        let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        b.iter(|| {
            for (mut pos, vel) in query.iter_mut(&mut world) {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/bevy", |b| {
        let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        b.iter(|| {
            for (mut pos, vel) in query.iter_mut(&mut world) {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/bevy", |b| {
        let mut world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        b.iter(|| {
            for (mut pos, vel) in query.iter_mut(&mut world) {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

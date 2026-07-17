use super::*;

pub(super) fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.bulk_add_entity((0..n).map(|_| suite_bundle()));
    world
}
pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/shipyard", |b| {
        let world = world_with_entities(SIMPLE_ENTITY_COUNT);
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/shipyard", |b| {
        let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/shipyard", |b| {
        let world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });
}

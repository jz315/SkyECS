use super::*;
use hecs::Archetype;

type MovementQuery<'a> = (&'a mut PositionComponent, &'a VelocityComponent);

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

pub(super) fn update_batched(world: &mut World) {
    for batch in world
        .query_mut::<MovementQuery<'_>>()
        .into_iter_batched(u32::MAX)
    {
        for (position, velocity) in batch {
            position.0 += velocity.0;
        }
    }
}

pub(super) fn matching_archetypes(world: &World, expected: usize) -> Vec<&Archetype> {
    let archetypes = world
        .archetypes()
        .filter(|archetype| archetype.satisfies::<MovementQuery<'_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        archetypes
            .iter()
            .map(|archetype| archetype.len() as usize)
            .sum::<usize>(),
        expected
    );
    archetypes
}

pub(super) fn update_archetype_columns(archetypes: &[&Archetype]) {
    for archetype in archetypes {
        let mut positions = archetype
            .get::<&mut PositionComponent>()
            .expect("matching archetype must contain PositionComponent");
        let velocities = archetype
            .get::<&VelocityComponent>()
            .expect("matching archetype must contain VelocityComponent");
        move_columns(&mut positions, &velocities);
    }
}

#[inline(never)]
fn move_columns(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/hecs", |b| {
        let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
        b.iter(|| {
            update_batched(&mut world);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/hecs", |b| {
        let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            update_batched(&mut world);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/hecs", |b| {
        let world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        let archetypes = matching_archetypes(&world, VERY_LARGE_ITERATION_ENTITY_COUNT);
        b.iter(|| {
            update_archetype_columns(&archetypes);
            black_box(&archetypes);
        });
    });
}

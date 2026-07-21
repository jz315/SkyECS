use criterion::{measurement::WallTime, BenchmarkGroup, Criterion};
use hecs::{Archetype, PreparedQuery, World};
use sky_ecs_comparison::common::{
    suite_bundle, PositionComponent, VelocityComponent, LARGE_ITERATION_ENTITY_COUNT,
    SIMPLE_ENTITY_COUNT, VERY_LARGE_ITERATION_ENTITY_COUNT,
};
use std::hint::black_box;
use std::time::Duration;

type MovementQuery<'a> = (&'a mut PositionComponent, &'a VelocityComponent);

fn world_with_entities(entity_count: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..entity_count).map(|_| suite_bundle()));
    world
}

fn configure_group<'a>(
    criterion: &'a mut Criterion,
    entity_count: usize,
) -> BenchmarkGroup<'a, WallTime> {
    let mut group = criterion.benchmark_group(format!("hecs_dense_candidates/{entity_count}"));
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    group
}

fn update_iter<'a>(iter: impl Iterator<Item = MovementQuery<'a>>) {
    for (position, velocity) in iter {
        position.0 += velocity.0;
    }
}

#[inline(never)]
fn update_columns(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

fn matching_archetypes(world: &World, expected: usize) -> Vec<&Archetype> {
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

fn bench_candidates(criterion: &mut Criterion, entity_count: usize) {
    let mut group = configure_group(criterion, entity_count);

    group.bench_function("prepared_shared", |bencher| {
        let world = world_with_entities(entity_count);
        let mut query = PreparedQuery::<MovementQuery<'_>>::default();
        assert_eq!(query.query(&world).iter().count(), entity_count);
        bencher.iter(|| {
            update_iter(query.query(&world).iter());
            black_box(&world);
        });
    });

    group.bench_function("prepared_unique", |bencher| {
        let mut world = world_with_entities(entity_count);
        let mut query = PreparedQuery::<MovementQuery<'_>>::default();
        assert_eq!(query.query_mut(&mut world).count(), entity_count);
        bencher.iter(|| {
            update_iter(query.query_mut(&mut world));
            black_box(&world);
        });
    });

    group.bench_function("world_unique", |bencher| {
        let mut world = world_with_entities(entity_count);
        assert_eq!(
            world.query_mut::<MovementQuery<'_>>().into_iter().count(),
            entity_count
        );
        bencher.iter(|| {
            update_iter(world.query_mut::<MovementQuery<'_>>().into_iter());
            black_box(&world);
        });
    });

    group.bench_function("prepared_view_unique", |bencher| {
        let mut world = world_with_entities(entity_count);
        let mut query = PreparedQuery::<MovementQuery<'_>>::default();
        assert_eq!(query.view_mut(&mut world).iter_mut().count(), entity_count);
        bencher.iter(|| {
            update_iter(query.view_mut(&mut world).iter_mut());
            black_box(&world);
        });
    });

    group.bench_function("world_batched_unique", |bencher| {
        let mut world = world_with_entities(entity_count);
        assert_eq!(
            world
                .query_mut::<MovementQuery<'_>>()
                .into_iter_batched(u32::MAX)
                .map(Iterator::count)
                .sum::<usize>(),
            entity_count
        );
        bencher.iter(|| {
            for batch in world
                .query_mut::<MovementQuery<'_>>()
                .into_iter_batched(u32::MAX)
            {
                update_iter(batch);
            }
            black_box(&world);
        });
    });

    group.bench_function("prepared_archetype_columns", |bencher| {
        let world = world_with_entities(entity_count);
        let archetypes = matching_archetypes(&world, entity_count);
        bencher.iter(|| {
            for archetype in &archetypes {
                let mut positions = archetype
                    .get::<&mut PositionComponent>()
                    .expect("matching archetype must contain PositionComponent");
                let velocities = archetype
                    .get::<&VelocityComponent>()
                    .expect("matching archetype must contain VelocityComponent");
                update_columns(&mut positions, &velocities);
            }
            black_box(&archetypes);
        });
    });

    group.finish();
}

pub fn bench_10k(criterion: &mut Criterion) {
    bench_candidates(criterion, SIMPLE_ENTITY_COUNT);
}

pub fn bench_100k(criterion: &mut Criterion) {
    bench_candidates(criterion, LARGE_ITERATION_ENTITY_COUNT);
}

pub fn bench_1m(criterion: &mut Criterion) {
    bench_candidates(criterion, VERY_LARGE_ITERATION_ENTITY_COUNT);
}

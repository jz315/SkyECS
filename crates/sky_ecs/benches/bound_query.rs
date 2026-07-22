#[path = "common.rs"]
mod common;

use common::{Position2D, Velocity2D};
use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::{Any, PreparedQuery, QueryData, With, World};
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNT: usize = 100_000;

#[derive(Clone, Copy)]
struct Active;

#[derive(Clone, Copy)]
struct Selected;

#[derive(Clone, Copy)]
struct HistoricalTag<const N: usize>;

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position2D,
    velocity: &'w Velocity2D,
}

fn populated_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..ENTITY_COUNT / 2).map(|index| {
        (
            Position2D {
                x: index as f32,
                y: 0.0,
            },
            Velocity2D { x: 1.0, y: 0.5 },
            Active,
        )
    }));
    world.spawn_batch((ENTITY_COUNT / 2..ENTITY_COUNT).map(|index| {
        (
            Position2D {
                x: index as f32,
                y: 0.0,
            },
            Velocity2D { x: 1.0, y: 0.5 },
            Selected,
        )
    }));
    world
}

fn churn_historical_storage<const N: usize>(world: &mut World) {
    let entity = world.spawn((Position2D { x: 0.0, y: 0.0 }, HistoricalTag::<N>));
    assert!(world.despawn(entity));
}

fn world_after_historical_schema_churn() -> World {
    let mut world = World::new();
    macro_rules! churn {
        ($($index:literal),+ $(,)?) => {
            $(churn_historical_storage::<$index>(&mut world);)+
        };
    }
    churn!(
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    );
    world.spawn((Position2D { x: 1.0, y: 2.0 },));
    world
}

#[inline(always)]
fn update(position: &mut Position2D, velocity: &Velocity2D) {
    position.x = black_box(position.x + velocity.x * 0.000_001);
    position.y = black_box(position.y + velocity.y * 0.000_001);
}

fn bench_bound_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("bound_query");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(20);

    let cache_world = populated_world();
    let _ = cache_world
        .query::<(&Position2D, &Velocity2D)>()
        .filter::<Any<(With<Active>, With<Selected>)>>()
        .cached_archetype_count();
    group.bench_function("world_cache_hit", |b| {
        b.iter(|| {
            black_box(
                cache_world
                    .query::<(&Position2D, &Velocity2D)>()
                    .filter::<Any<(With<Active>, With<Selected>)>>()
                    .cached_archetype_count(),
            )
        });
    });

    let mut bound_world = populated_world();
    group.bench_function("bound_tuple_for_each", |b| {
        b.iter(|| {
            bound_world
                .query_mut::<(&mut Position2D, &Velocity2D)>()
                .for_each(|(position, velocity)| update(position, velocity));
        });
    });

    let mut named_world = populated_world();
    group.bench_function("bound_named_for_each", |b| {
        b.iter(|| {
            named_world
                .query_mut::<Movement>()
                .for_each(|item| update(item.position, item.velocity));
        });
    });

    let mut prepared_world = populated_world();
    let mut prepared = PreparedQuery::<(&mut Position2D, &Velocity2D)>::new();
    group.bench_function("prepared_tuple_for_each", |b| {
        b.iter(|| {
            prepared.for_each(&mut prepared_world, |(position, velocity)| {
                update(position, velocity);
            });
        });
    });

    let clean_world = {
        let mut world = World::new();
        world.spawn((Position2D { x: 1.0, y: 2.0 },));
        world
    };
    let historical_world = world_after_historical_schema_churn();
    let mut clean_query = PreparedQuery::<&Position2D>::new();
    let mut historical_query = PreparedQuery::<&Position2D>::new();
    assert_eq!(clean_query.cached_archetype_count(), 0);
    assert_eq!(historical_query.cached_archetype_count(), 0);
    group.bench_function("one_active_clean_schema", |b| {
        b.iter(|| {
            clean_query.for_each(&clean_world, |position| {
                black_box(position);
            })
        });
    });
    group.bench_function("one_active_after_32_empty_storages", |b| {
        b.iter(|| {
            historical_query.for_each(&historical_world, |position| {
                black_box(position);
            })
        });
    });

    group.finish();
}

criterion_group!(benches, bench_bound_query);
criterion_main!(benches);

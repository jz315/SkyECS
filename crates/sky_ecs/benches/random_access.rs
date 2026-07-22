#[path = "common.rs"]
mod common;

use common::{Position2D, Velocity2D};
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use sky_ecs::{EntityId, PreparedEntityView, World};
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNTS: [usize; 2] = [10_000, 100_000];
const ORDER_COUNT: usize = 4;

fn deterministic_shuffle<T>(values: &mut [T], mut state: u64) {
    for upper in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.swap(upper, state as usize % (upper + 1));
    }
}

fn world_and_orders(entity_count: usize) -> (World, Vec<Vec<EntityId>>) {
    let mut world = World::new();
    let entities: Vec<_> = (0..entity_count)
        .map(|index| {
            world.spawn((
                Position2D {
                    x: index as f32,
                    y: index as f32 + 0.5,
                },
                Velocity2D { x: 1.0, y: 2.0 },
            ))
        })
        .collect();
    let orders = (0..ORDER_COUNT)
        .map(|order| {
            let mut shuffled = entities.clone();
            deterministic_shuffle(
                &mut shuffled,
                0x4d59_5df4_d0f3_3173 ^ (order as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
            );
            shuffled
        })
        .collect();
    (world, orders)
}

fn bench_reads(group: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>, count: usize) {
    group.bench_function("world_get", |bencher| {
        let (world, orders) = world_and_orders(count);
        let mut order = 0;
        bencher.iter(|| {
            for &entity in &orders[order % orders.len()] {
                black_box(world.get::<Position2D>(entity));
            }
            order += 1;
        });
    });

    group.bench_function("entity_accessor", |bencher| {
        let (world, orders) = world_and_orders(count);
        let positions = world.accessor::<Position2D>();
        let mut order = 0;
        bencher.iter(|| {
            for &entity in &orders[order % orders.len()] {
                black_box(positions.get(entity));
            }
            order += 1;
        });
    });

    group.bench_function("prepared_entity_access", |bencher| {
        let (world, orders) = world_and_orders(count);
        let plans: Vec<_> = orders
            .iter()
            .map(|entities| world.prepare_access::<Position2D>(entities).unwrap())
            .collect();
        let mut order = 0;
        bencher.iter(|| {
            for position in plans[order % plans.len()].iter() {
                black_box(position);
            }
            order += 1;
        });
    });

    group.bench_function("prepared_entity_view", |bencher| {
        let (world, orders) = world_and_orders(count);
        let mut prepared = PreparedEntityView::<&Position2D>::new();
        let positions = prepared.bind(&world);
        let mut order = 0;
        bencher.iter(|| {
            for &entity in &orders[order % orders.len()] {
                black_box(positions.get(entity));
            }
            order += 1;
        });
    });

    group.bench_function("prepare_access", |bencher| {
        let (world, orders) = world_and_orders(count);
        let mut order = 0;
        bencher.iter(|| {
            let plan = world
                .prepare_access::<Position2D>(&orders[order % orders.len()])
                .unwrap();
            black_box(plan.len());
            order += 1;
        });
    });
}

fn bench_writes(group: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>, count: usize) {
    group.bench_function("world_get_mut", |bencher| {
        let (mut world, orders) = world_and_orders(count);
        let mut order = 0;
        bencher.iter(|| {
            for &entity in &orders[order % orders.len()] {
                let position = world.get_mut::<Position2D>(entity).unwrap();
                position.x = black_box(position.x + 0.000_001);
            }
            order += 1;
        });
    });

    group.bench_function("entity_accessor_mut", |bencher| {
        let (mut world, orders) = world_and_orders(count);
        let mut positions = world.accessor_mut::<Position2D>();
        let mut order = 0;
        bencher.iter(|| {
            for &entity in &orders[order % orders.len()] {
                let position = positions.get_mut(entity).unwrap();
                position.x = black_box(position.x + 0.000_001);
            }
            order += 1;
        });
    });

    group.bench_function("prepared_entity_access_mut", |bencher| {
        let (mut world, orders) = world_and_orders(count);
        let mut positions = world.prepare_access_mut::<Position2D>(&orders[0]).unwrap();
        bencher.iter(|| {
            for position in positions.iter_mut() {
                position.x = black_box(position.x + 0.000_001);
            }
        });
    });

    group.bench_function("prepare_access_mut", |bencher| {
        let (mut world, orders) = world_and_orders(count);
        let mut order = 0;
        bencher.iter(|| {
            let plan = world
                .prepare_access_mut::<Position2D>(&orders[order % orders.len()])
                .unwrap();
            black_box(plan.len());
            order += 1;
        });
    });
}

fn bench_random_access(criterion: &mut Criterion) {
    for count in ENTITY_COUNTS {
        let mut group = criterion.benchmark_group(format!("random_access_{count}"));
        group.warm_up_time(Duration::from_millis(500));
        group.measurement_time(Duration::from_secs(2));
        group.sample_size(30);
        bench_reads(&mut group, count);
        bench_writes(&mut group, count);
        group.finish();
    }
}

criterion_group!(benches, bench_random_access);
criterion_main!(benches);

#[path = "common.rs"]
mod common;

use common::{Position2D, Velocity2D};
use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::World;
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNT: usize = 10_000;

fn deterministic_shuffle<T>(values: &mut [T]) {
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for upper in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.swap(upper, state as usize % (upper + 1));
    }
}

fn bench_random_access(c: &mut Criterion) {
    let mut world = World::new();
    let mut entities: Vec<_> = (0..ENTITY_COUNT)
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
    deterministic_shuffle(&mut entities);

    let mut group = c.benchmark_group("random_access_10k");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(30);

    group.bench_function("world_get", |b| {
        b.iter(|| {
            for &entity in &entities {
                black_box(world.get::<Position2D>(entity));
            }
        });
    });

    let positions = world.accessor::<Position2D>();
    group.bench_function("component_accessor", |b| {
        b.iter(|| {
            for &entity in &entities {
                black_box(positions.get(entity));
            }
        });
    });

    drop(positions);
    group.bench_function("world_get_mut", |b| {
        b.iter(|| {
            for &entity in &entities {
                let position = world.get_mut::<Position2D>(entity).unwrap();
                position.x = black_box(position.x + 0.000_001);
            }
        });
    });

    let mut positions = world.accessor_mut::<Position2D>();
    group.bench_function("component_accessor_mut", |b| {
        b.iter(|| {
            for &entity in &entities {
                let position = positions.get_mut(entity).unwrap();
                position.x = black_box(position.x + 0.000_001);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_random_access);
criterion_main!(benches);

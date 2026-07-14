use crate::common::*;
use crate::shared::sample_entities;
use bevy_ecs::entity::Entity as BevyEntity;
use bevy_ecs::query::QueryState;
use bevy_ecs::world::World;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use std::hint::black_box;

fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($world:ident; $($tag:ident),* $(,)?) => {
            $( for _ in 0..FRAGMENTED_ENTITIES_PER_VARIANT { $world.spawn(($tag(0.0), DataComponent(1.0))); } )*
        };
    }

    add_variant!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
    world
}

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

fn mixed_world() -> (World, Vec<BevyEntity>, Vec<BevyEntity>) {
    let mut world = World::new();
    let mut all_entities = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );
    let mut churn_entities = Vec::with_capacity(MIXED_FRAME_CHURN_COUNT);

    for _ in 0..MIXED_FRAME_MOVERS {
        let entity = world.spawn(mixed_mover_bundle()).id();
        if churn_entities.len() < MIXED_FRAME_CHURN_COUNT {
            churn_entities.push(entity);
        }
        all_entities.push(entity);
    }

    for _ in 0..MIXED_FRAME_ENEMIES {
        all_entities.push(world.spawn(mixed_enemy_bundle()).id());
    }

    for _ in 0..MIXED_FRAME_ALLIES {
        all_entities.push(world.spawn(mixed_ally_bundle()).id());
    }

    for _ in 0..MIXED_FRAME_HEAVY {
        all_entities.push(world.spawn(mixed_heavy_bundle()).id());
    }

    let random_entities = sample_entities(&all_entities, MIXED_FRAME_RANDOM_COUNT);
    (world, random_entities, churn_entities)
}

fn mixed_move_step(
    world: &mut World,
    move_query: &mut QueryState<(&mut PositionComponent, &VelocityComponent)>,
) {
    for (mut position, velocity) in move_query.iter_mut(world) {
        position.0 += velocity.0;
    }
}

fn mixed_health_step(
    world: &mut World,
    enemy_query: &mut QueryState<(&mut Health, &Damage)>,
    ally_query: &mut QueryState<(&mut Health, &Regen)>,
) {
    for (mut health, damage) in enemy_query.iter_mut(world) {
        health.0 -= damage.0;
    }

    for (mut health, regen) in ally_query.iter_mut(world) {
        health.0 += regen.0;
    }
}

fn mixed_heavy_step(
    world: &mut World,
    heavy_query: &mut QueryState<(&mut PositionComponent, &TransformComponent)>,
) {
    for (mut position, transform) in heavy_query.iter_mut(world) {
        let base = transform.0;
        let mut matrix = base;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = black_box(base)
                .invert()
                .expect("mixed-frame matrix should be invertible");
        }
        position.0 = matrix.transform_vector(position.0);
    }
}

fn mixed_random_step(world: &World, random_entities: &[BevyEntity]) {
    for &entity in random_entities {
        black_box(world.get::<PositionComponent>(entity));
    }
}

fn mixed_churn_step(world: &mut World, churn_entities: &[BevyEntity]) {
    for &entity in churn_entities {
        world.entity_mut(entity).insert(Health(100.0));
    }
    for &entity in churn_entities {
        world.entity_mut(entity).remove::<Health>();
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<BevyEntity>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(world.spawn(light_bundle()).id());
    }
    for &entity in spawned_entities.iter() {
        world.despawn(entity);
    }
}

fn run_mixed_frame(
    world: &mut World,
    move_query: &mut QueryState<(&mut PositionComponent, &VelocityComponent)>,
    enemy_query: &mut QueryState<(&mut Health, &Damage)>,
    ally_query: &mut QueryState<(&mut Health, &Regen)>,
    heavy_query: &mut QueryState<(&mut PositionComponent, &TransformComponent)>,
    random_entities: &[BevyEntity],
    churn_entities: &[BevyEntity],
    spawned_entities: &mut Vec<BevyEntity>,
) {
    mixed_move_step(world, move_query);
    mixed_health_step(world, enemy_query, ally_query);
    mixed_heavy_step(world, heavy_query);
    mixed_random_step(world, random_entities);
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_insert_10k/bevy", |b| {
        b.iter_batched_ref(
            World::new,
            |world| {
                world.spawn_batch((0..SIMPLE_ENTITY_COUNT).map(|_| suite_bundle()));
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/bevy", |b| {
        b.iter_batched_ref(
            World::new,
            |world| {
                for _ in 0..SIMPLE_ENTITY_COUNT {
                    world.spawn(suite_bundle());
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn validate_contract() {
    let mut world = world_with_entities(128);
    assert_eq!(
        world.query::<&PositionComponent>().iter(&world).count(),
        128
    );
    let mut count = 0;
    let mut checksum = 0.0;
    for (mut position, velocity) in world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .iter_mut(&mut world)
    {
        position.0 += velocity.0;
        count += 1;
        checksum += position.0.x;
    }
    assert_eq!(count, 128);
    assert_eq!(checksum, 256.0);

    let entity = world.spawn(light_bundle()).id();
    assert!(world.get::<PositionComponent>(entity).is_some());
    world.entity_mut(entity).insert(Health(100.0));
    assert!(world.get::<Health>(entity).is_some());
    world.entity_mut(entity).remove::<Health>();
    assert!(world.get::<Health>(entity).is_none());
    assert!(world.despawn(entity));
    assert!(!world.entities().contains(entity));

    let mut fragmented = fragmented_world();
    assert_eq!(
        fragmented
            .query::<&DataComponent>()
            .iter(&fragmented)
            .count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.query::<&PositionComponent>().iter(&mixed).count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    let mut move_query = mixed.query::<(&mut PositionComponent, &VelocityComponent)>();
    let mut enemy_query = mixed.query::<(&mut Health, &Damage)>();
    let mut ally_query = mixed.query::<(&mut Health, &Regen)>();
    let mut heavy_query = mixed.query::<(&mut PositionComponent, &TransformComponent)>();
    run_mixed_frame(
        &mut mixed,
        &mut move_query,
        &mut enemy_query,
        &mut ally_query,
        &mut heavy_query,
        &random,
        &churn,
        &mut spawned,
    );
    assert_eq!(
        mixed.query::<&PositionComponent>().iter(&mixed).count(),
        expected
    );
    assert!(mixed.get::<Health>(churn[0]).is_none());
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_10k/bevy", |b| {
        b.iter(|| {
            for (mut pos, vel) in query.iter_mut(&mut world) {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_x32/bevy", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                for (mut pos, vel) in query.iter_mut(&mut world) {
                    pos.0 += vel.0;
                }
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_100k/bevy", |b| {
        b.iter(|| {
            for (mut pos, vel) in query.iter_mut(&mut world) {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let mut world = fragmented_world();
    let mut query = world.query::<&mut DataComponent>();

    group.bench_function("fragmented_26x400/bevy", |b| {
        b.iter(|| {
            for mut data in query.iter_mut(&mut world) {
                data.0 *= 2.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = heavy_world();
    let mut query = world.query::<(&mut PositionComponent, &mut TransformComponent)>();

    group.bench_function("heavy/bevy", |b| {
        b.iter(|| {
            for (mut position, transform) in query.iter_mut(&mut world) {
                let base = transform.0;
                let mut matrix = base;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("base heavy matrix should be invertible");
                }
                position.0 = matrix.transform_vector(position.0);
            }
            black_box(&world);
        });
    });
}

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
        ("cold_1m", COLD_RANDOM_ENTITY_COUNT),
    ] {
        let mut world = World::new();
        let entities: Vec<_> = (0..count)
            .map(|_| world.spawn(light_bundle()).id())
            .collect();
        let orders = deterministic_orders(&entities);
        let mut order = 0;
        group.bench_function(format!("{name}/bevy"), |b| {
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                for &entity in entities {
                    black_box(world.get::<PositionComponent>(entity).unwrap());
                }
            });
        });
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/bevy", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(world.spawn(light_bundle()).id());
            }
            for &entity in &entities {
                assert!(world.despawn(entity));
            }
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/bevy", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_OP_COUNT)
            .map(|_| world.spawn(light_bundle()).id())
            .collect();

        b.iter(|| {
            for &entity in &entities {
                world.entity_mut(entity).insert(Health(100.0));
            }
            for &entity in &entities {
                world.entity_mut(entity).remove::<Health>();
            }
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (mut world, random_entities, churn_entities) = mixed_world();
    let mut move_query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
    let mut enemy_query = world.query::<(&mut Health, &Damage)>();
    let mut ally_query = world.query::<(&mut Health, &Regen)>();
    let mut heavy_query = world.query::<(&mut PositionComponent, &TransformComponent)>();
    let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    group.bench_function("frame/bevy", |b| {
        b.iter(|| {
            run_mixed_frame(
                &mut world,
                &mut move_query,
                &mut enemy_query,
                &mut ally_query,
                &mut heavy_query,
                &random_entities,
                &churn_entities,
                &mut spawned_entities,
            );
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    {
        let (mut world, _, _) = mixed_world();
        let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        group.bench_function("movement/bevy", |b| {
            b.iter(|| {
                mixed_move_step(&mut world, &mut query);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut enemy_query = world.query::<(&mut Health, &Damage)>();
        let mut ally_query = world.query::<(&mut Health, &Regen)>();
        group.bench_function("health/bevy", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    mixed_health_step(&mut world, &mut enemy_query, &mut ally_query);
                }
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut query = world.query::<(&mut PositionComponent, &TransformComponent)>();
        group.bench_function("heavy/bevy", |b| {
            b.iter(|| {
                mixed_heavy_step(&mut world, &mut query);
                black_box(&world);
            });
        });
    }

    {
        let (world, random_entities, _) = mixed_world();
        group.bench_function("random_access/bevy", |b| {
            b.iter(|| {
                mixed_random_step(&world, &random_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, churn_entities) = mixed_world();
        group.bench_function("structural_churn/bevy", |b| {
            b.iter(|| {
                mixed_churn_step(&mut world, &churn_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/bevy", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    mixed_spawn_step(&mut world, &mut spawned_entities);
                }
                black_box(&world);
            });
        });
    }
}

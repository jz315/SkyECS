use crate::common::{
    add_position_checksum, light_bundle, mixed_ally_bundle, mixed_enemy_bundle, mixed_heavy_bundle,
    mixed_mover_bundle, sample_entities, Damage, Health, PositionComponent, Regen,
    TransformComponent, VelocityComponent, MIXED_FRAME_ALLIES, MIXED_FRAME_CHURN_COUNT,
    MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_INVERT_COUNT, MIXED_FRAME_MOVERS,
    MIXED_FRAME_RANDOM_COUNT, MIXED_FRAME_SPAWN_COUNT, MIXED_PHASE_HEALTH_REPEAT,
    MIXED_PHASE_SPAWN_REPEAT,
};
use bevy_ecs::{entity::Entity as BevyEntity, query::QueryState, world::World};
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;

pub(super) fn mixed_world() -> (World, Vec<BevyEntity>, Vec<BevyEntity>) {
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
        let mut matrix = transform.0;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = matrix
                .invert()
                .expect("mixed-frame matrix should remain invertible");
        }
        position.0 = matrix.transform_vector(position.0);
    }
}

fn mixed_random_step(
    world: &World,
    random_query: &QueryState<&PositionComponent>,
    random_entities: &[BevyEntity],
) -> u64 {
    let mut checksum = 0_u64;
    for &entity in random_entities {
        let position = random_query
            .get_manual(world, entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, position);
    }
    checksum
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
        let removed = world.despawn(entity);
        debug_assert!(removed);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_mixed_frame(
    world: &mut World,
    move_query: &mut QueryState<(&mut PositionComponent, &VelocityComponent)>,
    enemy_query: &mut QueryState<(&mut Health, &Damage)>,
    ally_query: &mut QueryState<(&mut Health, &Regen)>,
    heavy_query: &mut QueryState<(&mut PositionComponent, &TransformComponent)>,
    random_query: &mut QueryState<&PositionComponent>,
    random_entities: &[BevyEntity],
    churn_entities: &[BevyEntity],
    spawned_entities: &mut Vec<BevyEntity>,
) -> u64 {
    mixed_move_step(world, move_query);
    mixed_health_step(world, enemy_query, ally_query);
    mixed_heavy_step(world, heavy_query);
    let checksum = mixed_random_step(world, random_query, random_entities);
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
    checksum
}
pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/bevy", |b| {
        let (mut world, random_entities, churn_entities) = mixed_world();
        let mut move_query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        let mut enemy_query = world.query::<(&mut Health, &Damage)>();
        let mut ally_query = world.query::<(&mut Health, &Regen)>();
        let mut heavy_query = world.query::<(&mut PositionComponent, &TransformComponent)>();
        let mut random_query = world.query::<&PositionComponent>();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        b.iter(|| {
            let checksum = run_mixed_frame(
                &mut world,
                &mut move_query,
                &mut enemy_query,
                &mut ally_query,
                &mut heavy_query,
                &mut random_query,
                &random_entities,
                &churn_entities,
                &mut spawned_entities,
            );
            black_box(checksum);
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("movement/bevy", |b| {
        let (mut world, _, _) = mixed_world();
        let mut query = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        b.iter(|| {
            mixed_move_step(&mut world, &mut query);
            black_box(&world);
        });
    });

    group.bench_function("health/bevy", |b| {
        let (mut world, _, _) = mixed_world();
        let mut enemy_query = world.query::<(&mut Health, &Damage)>();
        let mut ally_query = world.query::<(&mut Health, &Regen)>();
        b.iter(|| {
            for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                mixed_health_step(&mut world, &mut enemy_query, &mut ally_query);
            }
            black_box(&world);
        });
    });

    group.bench_function("heavy/bevy", |b| {
        let (mut world, _, _) = mixed_world();
        let mut query = world.query::<(&mut PositionComponent, &TransformComponent)>();
        b.iter(|| {
            mixed_heavy_step(&mut world, &mut query);
            black_box(&world);
        });
    });

    group.bench_function("random_access/bevy", |b| {
        let (mut world, random_entities, _) = mixed_world();
        let query = world.query::<&PositionComponent>();
        b.iter(|| {
            let checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
                add_position_checksum(
                    checksum,
                    query
                        .get_manual(&world, entity)
                        .expect("sampled entity must contain PositionComponent"),
                )
            });
            black_box(checksum);
            black_box(&world);
        });
    });

    group.bench_function("structural_churn/bevy", |b| {
        let (mut world, _, churn_entities) = mixed_world();
        b.iter(|| {
            mixed_churn_step(&mut world, &churn_entities);
            black_box(&world);
        });
    });

    group.bench_function("spawn_despawn/bevy", |b| {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        b.iter(|| {
            for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                mixed_spawn_step(&mut world, &mut spawned_entities);
            }
            black_box(&world);
        });
    });
}

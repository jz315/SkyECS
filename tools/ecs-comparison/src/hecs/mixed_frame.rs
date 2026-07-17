use super::dense_iteration::assert_prepared_count;
use crate::common::{
    add_position_checksum, light_bundle, mixed_ally_bundle, mixed_enemy_bundle, mixed_heavy_bundle,
    mixed_mover_bundle, sample_entities, Damage, Health, PositionComponent, Regen,
    TransformComponent, VelocityComponent, MIXED_FRAME_ALLIES, MIXED_FRAME_CHURN_COUNT,
    MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_INVERT_COUNT, MIXED_FRAME_MOVERS,
    MIXED_FRAME_RANDOM_COUNT, MIXED_FRAME_SPAWN_COUNT, MIXED_PHASE_HEALTH_REPEAT,
    MIXED_PHASE_SPAWN_REPEAT,
};
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BenchmarkGroup};
use hecs::{Entity as HecsEntity, PreparedQuery, World};
use std::hint::black_box;

pub(super) fn mixed_world() -> (World, Vec<HecsEntity>, Vec<HecsEntity>) {
    let mut world = World::new();
    let mut all_entities = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );
    let mut churn_entities = Vec::with_capacity(MIXED_FRAME_CHURN_COUNT);

    for _ in 0..MIXED_FRAME_MOVERS {
        let entity = world.spawn(mixed_mover_bundle());
        if churn_entities.len() < MIXED_FRAME_CHURN_COUNT {
            churn_entities.push(entity);
        }
        all_entities.push(entity);
    }

    for _ in 0..MIXED_FRAME_ENEMIES {
        all_entities.push(world.spawn(mixed_enemy_bundle()));
    }

    for _ in 0..MIXED_FRAME_ALLIES {
        all_entities.push(world.spawn(mixed_ally_bundle()));
    }

    for _ in 0..MIXED_FRAME_HEAVY {
        all_entities.push(world.spawn(mixed_heavy_bundle()));
    }

    let random_entities = sample_entities(&all_entities, MIXED_FRAME_RANDOM_COUNT);
    (world, random_entities, churn_entities)
}

fn mixed_move_step(
    world: &World,
    move_query: &mut PreparedQuery<(&mut PositionComponent, &VelocityComponent)>,
) {
    for (position, velocity) in move_query.query(world).iter() {
        position.0 += velocity.0;
    }
}

fn mixed_health_step(
    world: &World,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
) {
    for (health, damage) in enemy_query.query(world).iter() {
        health.0 -= damage.0;
    }

    for (health, regen) in ally_query.query(world).iter() {
        health.0 += regen.0;
    }
}

fn mixed_heavy_step(
    world: &World,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) {
    for (position, transform) in heavy_query.query(world).iter() {
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
    world: &mut World,
    random_query: &mut PreparedQuery<&PositionComponent>,
    random_entities: &[HecsEntity],
) -> u64 {
    let view = random_query.view_mut(world);
    let mut checksum = 0_u64;
    for &entity in random_entities {
        let position = view
            .get(entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, position);
    }
    checksum
}

fn mixed_churn_step(world: &mut World, churn_entities: &[HecsEntity]) {
    for &entity in churn_entities {
        let inserted = world.insert_one(entity, Health(100.0));
        debug_assert!(inserted.is_ok());
    }
    for &entity in churn_entities {
        let removed = world.remove_one::<Health>(entity);
        debug_assert!(removed.is_ok());
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<HecsEntity>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(world.spawn(light_bundle()));
    }
    for &entity in spawned_entities.iter() {
        let removed = world.despawn(entity);
        debug_assert!(removed.is_ok());
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_mixed_frame(
    world: &mut World,
    move_query: &mut PreparedQuery<(&mut PositionComponent, &VelocityComponent)>,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
    random_query: &mut PreparedQuery<&PositionComponent>,
    random_entities: &[HecsEntity],
    churn_entities: &[HecsEntity],
    spawned_entities: &mut Vec<HecsEntity>,
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
    group.bench_function("frame/hecs", |b| {
        let (mut world, random_entities, churn_entities) = mixed_world();
        let mut move_query =
            PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::default();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::default();
        let mut heavy_query =
            PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::default();
        let mut random_query = PreparedQuery::<&PositionComponent>::default();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        assert_prepared_count(
            &mut move_query,
            &world,
            MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
        );
        assert_prepared_count(&mut enemy_query, &world, MIXED_FRAME_ENEMIES);
        assert_prepared_count(&mut ally_query, &world, MIXED_FRAME_ALLIES);
        assert_prepared_count(&mut heavy_query, &world, MIXED_FRAME_HEAVY);
        assert_prepared_count(
            &mut random_query,
            &world,
            MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
        );
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
    group.bench_function("movement/hecs", |b| {
        let (world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        assert_prepared_count(
            &mut query,
            &world,
            MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
        );
        b.iter(|| {
            mixed_move_step(&world, &mut query);
            black_box(&world);
        });
    });

    group.bench_function("health/hecs", |b| {
        let (world, _, _) = mixed_world();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::default();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::default();
        assert_prepared_count(&mut enemy_query, &world, MIXED_FRAME_ENEMIES);
        assert_prepared_count(&mut ally_query, &world, MIXED_FRAME_ALLIES);
        b.iter(|| {
            for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                mixed_health_step(&world, &mut enemy_query, &mut ally_query);
            }
            black_box(&world);
        });
    });

    group.bench_function("heavy/hecs", |b| {
        let (world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::default();
        assert_prepared_count(&mut query, &world, MIXED_FRAME_HEAVY);
        b.iter(|| {
            mixed_heavy_step(&world, &mut query);
            black_box(&world);
        });
    });

    group.bench_function("random_access/hecs", |b| {
        let (mut world, random_entities, _) = mixed_world();
        let mut query = PreparedQuery::<&PositionComponent>::default();
        let view = query.view_mut(&mut world);
        b.iter(|| {
            let checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
                add_position_checksum(
                    checksum,
                    view.get(entity)
                        .expect("sampled entity must contain PositionComponent"),
                )
            });
            black_box(checksum);
        });
    });

    group.bench_function("structural_churn/hecs", |b| {
        let (mut world, _, churn_entities) = mixed_world();
        b.iter(|| {
            mixed_churn_step(&mut world, &churn_entities);
            black_box(&world);
        });
    });

    group.bench_function("spawn_despawn/hecs", |b| {
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

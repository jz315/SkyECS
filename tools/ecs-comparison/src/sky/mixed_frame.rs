use crate::common::{
    add_full_position_checksum, add_position_checksum, light_bundle, mixed_ally_bundle,
    mixed_enemy_bundle, mixed_heavy_bundle, mixed_mover_bundle, sample_entities, Damage, Health,
    PositionComponent, Regen, TransformComponent, VelocityComponent, MIXED_FRAME_ALLIES,
    MIXED_FRAME_CHURN_COUNT, MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_INVERT_COUNT,
    MIXED_FRAME_MOVERS, MIXED_FRAME_RANDOM_COUNT, MIXED_FRAME_SPAWN_COUNT,
    MIXED_PHASE_HEALTH_REPEAT, MIXED_PHASE_SPAWN_REPEAT,
};
use criterion::{measurement::WallTime, BenchmarkGroup};
use sky_ecs::{EntityId, PreparedQuery, World};
use std::hint::black_box;

pub(super) fn mixed_world() -> (World, Vec<EntityId>, Vec<EntityId>) {
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
    world: &mut World,
    move_query: &mut PreparedQuery<(&mut PositionComponent, &VelocityComponent)>,
) {
    move_query.for_each_chunk(&mut *world, |(positions, velocities)| {
        for (position, velocity) in positions.iter_mut().zip(velocities) {
            position.0 += velocity.0;
        }
    });
}

fn mixed_health_step(
    world: &mut World,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
) {
    enemy_query.for_each_chunk(&mut *world, |(healths, damage)| {
        for (health, damage) in healths.iter_mut().zip(damage) {
            health.0 -= damage.0;
        }
    });

    ally_query.for_each_chunk(&mut *world, |(healths, regen)| {
        for (health, regen) in healths.iter_mut().zip(regen) {
            health.0 += regen.0;
        }
    });
}

fn mixed_heavy_step(
    world: &mut World,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) -> u64 {
    let mut checksum = 0_u64;
    heavy_query.for_each_chunk(&mut *world, |(positions, transforms)| {
        checksum = checksum.wrapping_add(process_mixed_heavy_chunk(positions, transforms));
    });
    checksum
}

// Eight inversions make this kernel short enough that inlining wins back more
// call/loop overhead than a separate function boundary saves.
#[inline(always)]
fn process_mixed_heavy_chunk(
    positions: &mut [PositionComponent],
    transforms: &[TransformComponent],
) -> u64 {
    let mut checksum = 0_u64;
    for (position, transform) in positions.iter_mut().zip(transforms) {
        let mut matrix = transform.0;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = matrix.inverse();
        }
        position.0 = matrix.transform_vector(position.0);
        checksum = add_full_position_checksum(checksum, position);
    }
    checksum
}

fn mixed_random_step(world: &World, random_entities: &[EntityId]) -> u64 {
    let positions = world.accessor::<PositionComponent>();
    let mut checksum = 0_u64;
    for &entity in random_entities {
        let position = positions
            .get(entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, position);
    }
    checksum
}

fn mixed_churn_step(world: &mut World, churn_entities: &[EntityId]) {
    for &entity in churn_entities {
        let inserted = world.insert(entity, Health(100.0));
        debug_assert!(inserted);
    }
    for &entity in churn_entities {
        let removed = world.remove::<Health>(entity);
        debug_assert!(removed);
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<EntityId>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(world.spawn(light_bundle()));
    }
    for &entity in spawned_entities.iter() {
        let removed = world.despawn(entity);
        debug_assert!(removed);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_mixed_frame(
    world: &mut World,
    move_query: &mut PreparedQuery<(&mut PositionComponent, &VelocityComponent)>,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
    random_entities: &[EntityId],
    churn_entities: &[EntityId],
    spawned_entities: &mut Vec<EntityId>,
) -> u64 {
    mixed_move_step(world, move_query);
    mixed_health_step(world, enemy_query, ally_query);
    let heavy_checksum = mixed_heavy_step(world, heavy_query);
    let checksum = heavy_checksum.wrapping_add(mixed_random_step(world, random_entities));
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
    checksum
}
pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/sky", |b| {
        let (mut world, random_entities, churn_entities) = mixed_world();
        let mut move_query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::new();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::new();
        let mut heavy_query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        assert_eq!(
            move_query.count(&world),
            MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
        );
        assert_eq!(enemy_query.count(&world), MIXED_FRAME_ENEMIES);
        assert_eq!(ally_query.count(&world), MIXED_FRAME_ALLIES);
        assert_eq!(heavy_query.count(&world), MIXED_FRAME_HEAVY);
        b.iter(|| {
            let checksum = run_mixed_frame(
                &mut world,
                &mut move_query,
                &mut enemy_query,
                &mut ally_query,
                &mut heavy_query,
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
    group.bench_function("movement/sky", |b| {
        let (mut world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        assert_eq!(
            query.count(&world),
            MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
        );
        b.iter(|| {
            mixed_move_step(&mut world, &mut query);
            black_box(&world);
        });
    });

    group.bench_function("health/sky", |b| {
        let (mut world, _, _) = mixed_world();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::new();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::new();
        assert_eq!(enemy_query.count(&world), MIXED_FRAME_ENEMIES);
        assert_eq!(ally_query.count(&world), MIXED_FRAME_ALLIES);
        b.iter(|| {
            for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                mixed_health_step(&mut world, &mut enemy_query, &mut ally_query);
            }
            black_box(&world);
        });
    });

    group.bench_function("heavy/sky", |b| {
        let (mut world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), MIXED_FRAME_HEAVY);
        b.iter(|| {
            let checksum = mixed_heavy_step(&mut world, &mut query);
            black_box(checksum);
            black_box(&world);
        });
    });

    group.bench_function("random_access/sky", |b| {
        let (world, random_entities, _) = mixed_world();
        let positions = world.accessor::<PositionComponent>();
        b.iter(|| {
            let checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
                add_position_checksum(
                    checksum,
                    positions
                        .get(entity)
                        .expect("sampled entity must contain PositionComponent"),
                )
            });
            black_box(checksum);
            black_box(&world);
        });
    });

    group.bench_function("structural_churn/sky", |b| {
        let (mut world, _, churn_entities) = mixed_world();
        b.iter(|| {
            mixed_churn_step(&mut world, &churn_entities);
            black_box(&world);
        });
    });

    group.bench_function("spawn_despawn/sky", |b| {
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

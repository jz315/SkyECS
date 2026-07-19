use super::structural_changes::spawn_light_one;
use super::*;

pub(super) fn mixed_world() -> (World, Vec<Entity>, Vec<Entity>) {
    let mut world = World::default();
    let mut all_entities = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );

    let movers = world.spawn_batch(MOVE_MASK, MIXED_FRAME_MOVERS, |table, index| {
        let (position, velocity) = mixed_mover_bundle();
        table.position[index] = position;
        table.velocity[index] = velocity;
    });
    let churn_entities = movers[..MIXED_FRAME_CHURN_COUNT].to_vec();
    all_entities.extend(movers);

    all_entities.extend(world.spawn_batch(
        MOVE_MASK | HEALTH_MASK | DAMAGE_MASK | IS_ENEMY_MASK,
        MIXED_FRAME_ENEMIES,
        |table, index| {
            let (position, velocity, health, damage, is_enemy) = mixed_enemy_bundle();
            table.position[index] = position;
            table.velocity[index] = velocity;
            table.health[index] = health;
            table.damage[index] = damage;
            table.is_enemy[index] = is_enemy;
        },
    ));

    all_entities.extend(world.spawn_batch(
        MOVE_MASK | HEALTH_MASK | REGEN_MASK | IS_ALLY_MASK,
        MIXED_FRAME_ALLIES,
        |table, index| {
            let (position, velocity, health, regen, is_ally) = mixed_ally_bundle();
            table.position[index] = position;
            table.velocity[index] = velocity;
            table.health[index] = health;
            table.regen[index] = regen;
            table.is_ally[index] = is_ally;
        },
    ));

    all_entities.extend(world.spawn_batch(
        MOVE_MASK | TRANSFORM_MASK,
        MIXED_FRAME_HEAVY,
        |table, index| {
            let (transform, position, velocity) = mixed_heavy_bundle();
            table.transform[index] = transform;
            table.position[index] = position;
            table.velocity[index] = velocity;
        },
    ));

    let random_entities = sample_entities(&all_entities, MIXED_FRAME_RANDOM_COUNT);
    (world, random_entities, churn_entities)
}

pub(super) fn warm_query(world: &mut World, mask: u64) {
    world.for_each_mut(mask, 0, |_entity, _table, _index| {});
}

pub(super) fn warm_mixed_queries(world: &mut World) {
    for mask in [MOVE_MASK, ENEMY_HEALTH_MASK, ALLY_HEALTH_MASK, HEAVY_MASK] {
        warm_query(world, mask);
    }
}

pub(super) fn mixed_move_step(world: &mut World) {
    world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
        table.position[index].0 += table.velocity[index].0;
    });
}

pub(super) fn mixed_health_step(world: &mut World) {
    world.for_each_mut(ENEMY_HEALTH_MASK, 0, |_entity, table, index| {
        table.health[index].0 -= table.damage[index].0;
    });
    world.for_each_mut(ALLY_HEALTH_MASK, 0, |_entity, table, index| {
        table.health[index].0 += table.regen[index].0;
    });
}

pub(super) fn mixed_heavy_step(world: &mut World) -> u64 {
    let mut checksum = 0_u64;
    world.for_each_mut(HEAVY_MASK, 0, |_entity, table, index| {
        let mut matrix = table.transform[index].0;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = matrix.inverse();
        }
        table.position[index].0 = matrix.transform_vector(table.position[index].0);
        checksum = add_full_position_checksum(checksum, &table.position[index]);
    });
    checksum
}

pub(super) fn mixed_random_step(world: &World, random_entities: &[Entity]) -> u64 {
    let mut checksum = 0_u64;
    for &entity in random_entities {
        let position = world
            .get_position(entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, position);
    }
    checksum
}

pub(super) fn mixed_churn_step(world: &mut World, churn_entities: &[Entity]) {
    for &entity in churn_entities {
        world.set_health(entity, Health(100.0));
    }
    for &entity in churn_entities {
        let removed = world.remove_health(entity);
        debug_assert!(removed);
    }
}

pub(super) fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<Entity>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(spawn_light_one(world));
    }
    for &entity in spawned_entities.iter() {
        let despawned = world.despawn_entities(std::slice::from_ref(&entity));
        debug_assert_eq!(despawned.as_slice(), [entity]);
    }
}

pub(super) fn run_mixed_frame(
    world: &mut World,
    random_entities: &[Entity],
    churn_entities: &[Entity],
    spawned_entities: &mut Vec<Entity>,
) -> u64 {
    mixed_move_step(world);
    mixed_health_step(world);
    let heavy_checksum = mixed_heavy_step(world);
    let checksum = heavy_checksum.wrapping_add(mixed_random_step(world, random_entities));
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
    checksum
}
pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/freecs", |b| {
        let (mut world, random_entities, churn_entities) = mixed_world();
        warm_mixed_queries(&mut world);
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        b.iter(|| {
            let checksum = run_mixed_frame(
                &mut world,
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
    group.bench_function("movement/freecs", |b| {
        let (mut world, _, _) = mixed_world();
        warm_query(&mut world, MOVE_MASK);
        b.iter(|| {
            mixed_move_step(&mut world);
            black_box(&world);
        });
    });

    group.bench_function("health/freecs", |b| {
        let (mut world, _, _) = mixed_world();
        warm_query(&mut world, ENEMY_HEALTH_MASK);
        warm_query(&mut world, ALLY_HEALTH_MASK);
        b.iter(|| {
            for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                mixed_health_step(&mut world);
            }
            black_box(&world);
        });
    });

    group.bench_function("heavy/freecs", |b| {
        let (mut world, _, _) = mixed_world();
        warm_query(&mut world, HEAVY_MASK);
        b.iter(|| {
            let checksum = mixed_heavy_step(&mut world);
            black_box(checksum);
            black_box(&world);
        });
    });

    group.bench_function("random_access/freecs", |b| {
        let (world, random_entities, _) = mixed_world();
        b.iter(|| {
            let checksum = mixed_random_step(&world, &random_entities);
            black_box(checksum);
            black_box(&world);
        });
    });

    group.bench_function("structural_churn/freecs", |b| {
        let (mut world, _, churn_entities) = mixed_world();
        b.iter(|| {
            mixed_churn_step(&mut world, &churn_entities);
            black_box(&world);
        });
    });

    group.bench_function("spawn_despawn/freecs", |b| {
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

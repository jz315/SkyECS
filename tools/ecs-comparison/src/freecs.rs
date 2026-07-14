use crate::common::*;
use crate::shared::sample_entities;
use ::freecs::{ecs, Entity};
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use std::hint::black_box;

macro_rules! define_fragment_components {
    ($($name:ident),+ $(,)?) => {
        $(
            #[repr(transparent)]
            #[derive(Clone, Copy, Default)]
            pub struct $name(pub f32);
        )+
    };
}

define_fragment_components!(
    FragmentA, FragmentB, FragmentC, FragmentD, FragmentE, FragmentF, FragmentG, FragmentH,
    FragmentI, FragmentJ, FragmentK, FragmentL, FragmentM, FragmentN, FragmentO, FragmentP,
    FragmentQ, FragmentR, FragmentS, FragmentT, FragmentU, FragmentV, FragmentW, FragmentX,
    FragmentY, FragmentZ,
);

ecs! {
    World {
        transform: TransformComponent => TRANSFORM_MASK,
        position: PositionComponent => POSITION_MASK,
        rotation: RotationComponent => ROTATION_MASK,
        velocity: VelocityComponent => VELOCITY_MASK,
        data: DataComponent => DATA_MASK,
        health: Health => HEALTH_MASK,
        damage: Damage => DAMAGE_MASK,
        regen: Regen => REGEN_MASK,
        is_enemy: IsEnemy => IS_ENEMY_MASK,
        is_ally: IsAlly => IS_ALLY_MASK,
        fragment_a: FragmentA => A_MASK,
        fragment_b: FragmentB => B_MASK,
        fragment_c: FragmentC => C_MASK,
        fragment_d: FragmentD => D_MASK,
        fragment_e: FragmentE => E_MASK,
        fragment_f: FragmentF => F_MASK,
        fragment_g: FragmentG => G_MASK,
        fragment_h: FragmentH => H_MASK,
        fragment_i: FragmentI => I_MASK,
        fragment_j: FragmentJ => J_MASK,
        fragment_k: FragmentK => K_MASK,
        fragment_l: FragmentL => L_MASK,
        fragment_m: FragmentM => M_MASK,
        fragment_n: FragmentN => N_MASK,
        fragment_o: FragmentO => O_MASK,
        fragment_p: FragmentP => P_MASK,
        fragment_q: FragmentQ => Q_MASK,
        fragment_r: FragmentR => R_MASK,
        fragment_s: FragmentS => S_MASK,
        fragment_t: FragmentT => T_MASK,
        fragment_u: FragmentU => U_MASK,
        fragment_v: FragmentV => V_MASK,
        fragment_w: FragmentW => W_MASK,
        fragment_x: FragmentX => X_MASK,
        fragment_y: FragmentY => Y_MASK,
        fragment_z: FragmentZ => Z_MASK,
    }
    Resources {}
}

const SUITE_MASK: u64 = TRANSFORM_MASK | POSITION_MASK | ROTATION_MASK | VELOCITY_MASK;
const LIGHT_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
const MOVE_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
const ENEMY_HEALTH_MASK: u64 = HEALTH_MASK | DAMAGE_MASK;
const ALLY_HEALTH_MASK: u64 = HEALTH_MASK | REGEN_MASK;
const HEAVY_MASK: u64 = POSITION_MASK | TRANSFORM_MASK;

fn spawn_suite_batch(world: &mut World, count: usize) -> Vec<Entity> {
    world.spawn_batch(SUITE_MASK, count, |table, index| {
        let (transform, position, rotation, velocity) = suite_bundle();
        table.transform[index] = transform;
        table.position[index] = position;
        table.rotation[index] = rotation;
        table.velocity[index] = velocity;
    })
}

fn spawn_suite_one(world: &mut World) -> Entity {
    spawn_suite_batch(world, 1)
        .pop()
        .expect("FreeCS should return the spawned entity")
}

fn spawn_light_batch(world: &mut World, count: usize) -> Vec<Entity> {
    world.spawn_batch(LIGHT_MASK, count, |table, index| {
        let (position, velocity) = light_bundle();
        table.position[index] = position;
        table.velocity[index] = velocity;
    })
}

fn spawn_light_one(world: &mut World) -> Entity {
    spawn_light_batch(world, 1)
        .pop()
        .expect("FreeCS should return the spawned entity")
}

fn world_with_entities(count: usize) -> World {
    let mut world = World::default();
    spawn_suite_batch(&mut world, count);
    world
}

fn fragmented_world() -> World {
    let mut world = World::default();

    for component_mask in [
        A_MASK, B_MASK, C_MASK, D_MASK, E_MASK, F_MASK, G_MASK, H_MASK, I_MASK, J_MASK, K_MASK,
        L_MASK, M_MASK, N_MASK, O_MASK, P_MASK, Q_MASK, R_MASK, S_MASK, T_MASK, U_MASK, V_MASK,
        W_MASK, X_MASK, Y_MASK, Z_MASK,
    ] {
        world.spawn_batch(
            component_mask | DATA_MASK,
            FRAGMENTED_ENTITIES_PER_VARIANT,
            |table, index| {
                table.data[index] = DataComponent(1.0);
            },
        );
    }

    world
}

fn heavy_world() -> World {
    let mut world = World::default();
    world.spawn_batch(SUITE_MASK, HEAVY_ENTITY_COUNT, |table, index| {
        let (transform, position, rotation, velocity) = heavy_bundle();
        table.transform[index] = transform;
        table.position[index] = position;
        table.rotation[index] = rotation;
        table.velocity[index] = velocity;
    });
    world
}

fn mixed_world() -> (World, Vec<Entity>, Vec<Entity>) {
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

fn warm_query(world: &mut World, mask: u64) {
    world.for_each_mut(mask, 0, |_entity, _table, _index| {});
}

fn warm_mixed_queries(world: &mut World) {
    for mask in [MOVE_MASK, ENEMY_HEALTH_MASK, ALLY_HEALTH_MASK, HEAVY_MASK] {
        warm_query(world, mask);
    }
}

fn mixed_move_step(world: &mut World) {
    world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
        table.position[index].0 += table.velocity[index].0;
    });
}

fn mixed_health_step(world: &mut World) {
    world.for_each_mut(ENEMY_HEALTH_MASK, 0, |_entity, table, index| {
        table.health[index].0 -= table.damage[index].0;
    });
    world.for_each_mut(ALLY_HEALTH_MASK, 0, |_entity, table, index| {
        table.health[index].0 += table.regen[index].0;
    });
}

fn mixed_heavy_step(world: &mut World) {
    world.for_each_mut(HEAVY_MASK, 0, |_entity, table, index| {
        let base = table.transform[index].0;
        let mut matrix = base;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = black_box(base)
                .invert()
                .expect("mixed-frame matrix should be invertible");
        }
        table.position[index].0 = matrix.transform_vector(table.position[index].0);
    });
}

fn mixed_random_step(world: &World, random_entities: &[Entity]) {
    for &entity in random_entities {
        black_box(world.get_position(entity));
    }
}

fn mixed_churn_step(world: &mut World, churn_entities: &[Entity]) {
    for &entity in churn_entities {
        world.set_health(entity, Health(100.0));
    }
    for &entity in churn_entities {
        world.remove_health(entity);
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<Entity>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(spawn_light_one(world));
    }
    black_box(world.despawn_entities(spawned_entities));
}

fn run_mixed_frame(
    world: &mut World,
    random_entities: &[Entity],
    churn_entities: &[Entity],
    spawned_entities: &mut Vec<Entity>,
) {
    mixed_move_step(world);
    mixed_health_step(world);
    mixed_heavy_step(world);
    mixed_random_step(world, random_entities);
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
}

pub fn validate_contract() {
    let mut world = world_with_entities(128);
    assert_eq!(world.entity_count(), 128);
    let mut count = 0;
    let mut checksum = 0.0;
    world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
        table.position[index].0 += table.velocity[index].0;
        count += 1;
        checksum += table.position[index].0.x;
    });
    assert_eq!(count, 128);
    assert_eq!(checksum, 256.0);

    let entity = spawn_light_one(&mut world);
    assert!(world.contains_entity(entity));
    assert!(world.get_position(entity).is_some());
    world.set_health(entity, Health(100.0));
    assert!(world.get_health(entity).is_some());
    world.remove_health(entity);
    assert!(world.get_health(entity).is_none());
    assert_eq!(world.despawn_entities(&[entity]), vec![entity]);
    assert!(!world.contains_entity(entity));

    let mut fragmented = fragmented_world();
    let mut fragmented_count = 0;
    fragmented.for_each_mut(DATA_MASK, 0, |_, _, _| fragmented_count += 1);
    assert_eq!(
        fragmented_count,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.entity_count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    run_mixed_frame(&mut mixed, &random, &churn, &mut spawned);
    assert_eq!(mixed.entity_count(), expected);
    assert!(mixed.get_health(churn[0]).is_none());
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_insert_10k/freecs", |b| {
        b.iter_batched_ref(
            World::default,
            |world| {
                black_box(spawn_suite_batch(world, SIMPLE_ENTITY_COUNT));
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/freecs", |b| {
        b.iter_batched_ref(
            World::default,
            |world| {
                for _ in 0..SIMPLE_ENTITY_COUNT {
                    black_box(spawn_suite_one(world));
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    warm_query(&mut world, MOVE_MASK);

    group.bench_function("simple_10k/freecs", |b| {
        b.iter(|| {
            world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    warm_query(&mut world, MOVE_MASK);

    group.bench_function("simple_x32/freecs", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                    table.position[index].0 += table.velocity[index].0;
                });
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    warm_query(&mut world, MOVE_MASK);

    group.bench_function("simple_100k/freecs", |b| {
        b.iter(|| {
            world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let mut world = fragmented_world();
    warm_query(&mut world, DATA_MASK);

    group.bench_function("fragmented_26x400/freecs", |b| {
        b.iter(|| {
            world.for_each_mut(DATA_MASK, 0, |_entity, table, index| {
                table.data[index].0 *= 2.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = heavy_world();
    warm_query(&mut world, HEAVY_MASK);

    group.bench_function("heavy/freecs", |b| {
        b.iter(|| {
            world.for_each_mut(HEAVY_MASK, 0, |_entity, table, index| {
                let base = table.transform[index].0;
                let mut matrix = base;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("base heavy matrix should be invertible");
                }
                table.position[index].0 = matrix.transform_vector(table.position[index].0);
            });
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
        let mut world = World::default();
        let entities = spawn_light_batch(&mut world, count);
        let orders = deterministic_orders(&entities);
        let mut order = 0;
        group.bench_function(format!("{name}/freecs"), |b| {
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                for &entity in entities {
                    black_box(world.get_position(entity).unwrap());
                }
            });
        });
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/freecs", |b| {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(spawn_light_one(&mut world));
            }
            black_box(world.despawn_entities(&entities));
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/freecs", |b| {
        let mut world = World::default();
        let entities = spawn_light_batch(&mut world, ENTITY_OP_COUNT);

        b.iter(|| {
            for &entity in &entities {
                world.set_health(entity, Health(100.0));
            }
            for &entity in &entities {
                world.remove_health(entity);
            }
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (mut world, random_entities, churn_entities) = mixed_world();
    warm_mixed_queries(&mut world);
    let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    group.bench_function("frame/freecs", |b| {
        b.iter(|| {
            run_mixed_frame(
                &mut world,
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
        warm_query(&mut world, MOVE_MASK);
        group.bench_function("movement/freecs", |b| {
            b.iter(|| {
                mixed_move_step(&mut world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        warm_query(&mut world, ENEMY_HEALTH_MASK);
        warm_query(&mut world, ALLY_HEALTH_MASK);
        group.bench_function("health/freecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    mixed_health_step(&mut world);
                }
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        warm_query(&mut world, HEAVY_MASK);
        group.bench_function("heavy/freecs", |b| {
            b.iter(|| {
                mixed_heavy_step(&mut world);
                black_box(&world);
            });
        });
    }

    {
        let (world, random_entities, _) = mixed_world();
        group.bench_function("random_access/freecs", |b| {
            b.iter(|| {
                mixed_random_step(&world, &random_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, churn_entities) = mixed_world();
        group.bench_function("structural_churn/freecs", |b| {
            b.iter(|| {
                mixed_churn_step(&mut world, &churn_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/freecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    mixed_spawn_step(&mut world, &mut spawned_entities);
                }
                black_box(&world);
            });
        });
    }
}

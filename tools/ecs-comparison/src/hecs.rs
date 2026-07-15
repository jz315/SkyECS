use crate::common::*;
use crate::shared::sample_entities;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use hecs::{Entity as HecsEntity, PreparedQuery, World};
use std::hint::black_box;

fn prepared_insert_world() -> World {
    let mut world = World::new();
    // `reserve(0)` registers the exact bundle archetype without reserving rows.
    world.reserve::<SuiteBundle>(0);
    world
}

fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($world:ident; $($tag:ident),* $(,)?) => {
            $( $world.spawn_batch((0..FRAGMENTED_ENTITIES_PER_VARIANT).map(|_| ($tag(0.0), DataComponent(1.0)))); )*
        };
    }

    add_variant!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
    world
}

fn random_fragmented_world(component_count: usize, entity_count: usize) -> (World, usize) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let expected = random_fragment_match_count(&masks);
    let mut world = World::new();

    for mask in masks {
        let entity = world.spawn(());
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    world
                        .insert_one(entity, $component(10.0))
                        .expect("random component insertion should succeed");
                }
            };
        }
        component!(0, A);
        component!(1, B);
        component!(2, C);
        component!(3, D);
        component!(4, E);
        component!(5, F);
        component!(6, G);
        component!(7, H);
        component!(8, I);
        component!(9, J);
        component!(10, K);
        component!(11, L);
        component!(12, M);
        component!(13, N);
        component!(14, O);
        component!(15, P);
    }

    (world, expected)
}

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

fn mixed_world() -> (World, Vec<HecsEntity>, Vec<HecsEntity>) {
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

fn mixed_random_step(world: &World, random_entities: &[HecsEntity]) -> u64 {
    let mut checksum = 0_u64;
    for &entity in random_entities {
        let position = world
            .get::<&PositionComponent>(entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, &position);
    }
    checksum
}

fn mixed_churn_step(world: &mut World, churn_entities: &[HecsEntity]) {
    for &entity in churn_entities {
        world
            .insert_one(entity, Health(100.0))
            .expect("mixed-frame entity must be alive and lack Health");
    }
    for &entity in churn_entities {
        world
            .remove_one::<Health>(entity)
            .expect("mixed-frame entity must be alive and contain Health");
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<HecsEntity>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(world.spawn(light_bundle()));
    }
    for &entity in spawned_entities.iter() {
        world
            .despawn(entity)
            .expect("newly spawned mixed-frame entity must be alive");
    }
}

fn run_mixed_frame(
    world: &mut World,
    move_query: &mut PreparedQuery<(&mut PositionComponent, &VelocityComponent)>,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
    random_entities: &[HecsEntity],
    churn_entities: &[HecsEntity],
    spawned_entities: &mut Vec<HecsEntity>,
) -> u64 {
    mixed_move_step(world, move_query);
    mixed_health_step(world, enemy_query, ally_query);
    mixed_heavy_step(world, heavy_query);
    let checksum = mixed_random_step(world, random_entities);
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
    checksum
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
    group.bench_function("bulk_insert_10k/hecs", |b| {
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                drop(world.spawn_batch(bundles.iter().copied()));
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/hecs", |b| {
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                for &bundle in &bundles {
                    world.spawn(bundle);
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn validate_contract() {
    let construction_world = prepared_insert_world();
    assert_eq!(construction_world.len(), 0);

    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    assert_eq!(world.len(), CONTRACT_ENTITY_COUNT as u32);
    let mut count = 0;
    let mut checksum = 0.0;
    for (position, velocity) in world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .iter()
    {
        position.0 += velocity.0;
        count += 1;
        checksum += position.0.x;
    }
    assert_eq!(count, CONTRACT_ENTITY_COUNT);
    assert_eq!(checksum, 256.0);

    let entity = world.spawn(light_bundle());
    assert!(world.get::<&PositionComponent>(entity).is_ok());
    world.insert_one(entity, Health(100.0)).unwrap();
    assert!(world.get::<&Health>(entity).is_ok());
    world.remove_one::<Health>(entity).unwrap();
    assert!(world.get::<&Health>(entity).is_err());
    world.despawn(entity).unwrap();
    assert!(!world.contains(entity));
    assert!(world.get::<&PositionComponent>(entity).is_err());

    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.spawn(light_bundle()))
        .collect();
    let mut random_query = PreparedQuery::<&PositionComponent>::default();
    let random_view = random_query.view_mut(&mut random_world);
    let random_checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
        add_position_checksum(
            checksum,
            random_view
                .get(entity)
                .expect("contract entity must be readable through PreparedView"),
        )
    });
    assert_eq!(
        random_checksum,
        position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
    );

    let fragmented = fragmented_world();
    assert_eq!(
        fragmented.query::<&DataComponent>().iter().count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (random_fragmented, expected) =
            random_fragmented_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        let mut matched = 0;
        let mut values = 0.0;
        for (a, b, c, d) in random_fragmented.query::<(&A, &B, &C, &D)>().iter() {
            matched += 1;
            values += a.0 + b.0 + c.0 + d.0;
        }
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * 40.0);
    }

    let base_count = world.len();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    assert_eq!(world.len(), base_count + ENTITY_OP_COUNT as u32);
    for &entity in &entity_ops {
        world.despawn(entity).unwrap();
    }
    assert_eq!(world.len(), base_count);
    assert!(entity_ops.iter().all(|&entity| !world.contains(entity)));

    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.len();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    for &entity in &churn {
        mixed.insert_one(entity, Health(100.0)).unwrap();
    }
    assert!(churn
        .iter()
        .all(|&entity| mixed.get::<&Health>(entity).is_ok()));
    for &entity in &churn {
        mixed.remove_one::<Health>(entity).unwrap();
    }

    let random_checksum = run_mixed_frame(
        &mut mixed,
        &mut PreparedQuery::default(),
        &mut PreparedQuery::default(),
        &mut PreparedQuery::default(),
        &mut PreparedQuery::default(),
        &random,
        &churn,
        &mut spawned,
    );
    assert_ne!(random_checksum, 0);
    assert_eq!(mixed.len(), expected);
    assert!(mixed.get::<&Health>(churn[0]).is_err());
    assert!(spawned.iter().all(|&entity| !mixed.contains(entity)));

    let mut position_count = 0;
    let mut position_sum = 0.0;
    for position in mixed.query::<&PositionComponent>().iter() {
        position_count += 1;
        position_sum += position.0.x;
    }
    assert_eq!(
        position_count,
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
    );
    assert_approx_eq(position_sum, 18_500.0);

    let mut health_count = 0;
    let mut health_sum = 0.0;
    for health in mixed.query::<&Health>().iter() {
        health_count += 1;
        health_sum += health.0;
    }
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();

    group.bench_function("simple_10k/hecs", |b| {
        b.iter(|| {
            for (pos, vel) in query.query(&world).iter() {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();

    group.bench_function("simple_x32/hecs", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                for (pos, vel) in query.query(&world).iter() {
                    pos.0 += vel.0;
                }
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();

    group.bench_function("simple_100k/hecs", |b| {
        b.iter(|| {
            for (pos, vel) in query.query(&world).iter() {
                pos.0 += vel.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let world = fragmented_world();
    let mut query = PreparedQuery::<&mut DataComponent>::default();

    group.bench_function("fragmented_26x400/hecs", |b| {
        b.iter(|| {
            for data in query.query(&world).iter() {
                data.0 = -data.0;
            }
            black_box(&world);
        });
    });
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (world, expected) =
            random_fragmented_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
        let mut query = PreparedQuery::<(hecs::Entity, &A, &B, &C, &D)>::default();
        assert_eq!(query.query(&world).iter().count(), expected);

        group.bench_function(
            format!("random_{component_count}_components_4_terms/hecs"),
            |b| {
                let mut checksum = 0_u64;
                b.iter(|| {
                    query.query(&world).iter().for_each(|(entity, a, b, c, d)| {
                        checksum = checksum
                            .wrapping_add(entity.to_bits().get())
                            .wrapping_add(a.0 as u64)
                            .wrapping_add(b.0 as u64)
                            .wrapping_add(c.0 as u64)
                            .wrapping_add(d.0 as u64);
                    });
                });
                black_box(checksum);
            },
        );
    }
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = heavy_world();
    let mut query = PreparedQuery::<(&mut PositionComponent, &mut TransformComponent)>::default();

    group.bench_function("heavy/hecs", |b| {
        b.iter(|| {
            for (position, transform) in query.query(&world).iter() {
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
        let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
        let orders = deterministic_orders(&entities);
        let mut query = PreparedQuery::<&PositionComponent>::default();
        let view = query.view_mut(&mut world);
        let mut order = 0;
        group.bench_function(format!("{name}/hecs"), |b| {
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = view
                        .get(entity)
                        .expect("random-access entity must contain PositionComponent");
                    checksum = add_position_checksum(checksum, position);
                }
                black_box(checksum);
            });
        });
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/hecs", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(world.spawn(light_bundle()));
            }
            for &entity in &entities {
                world
                    .despawn(entity)
                    .expect("newly spawned benchmark entity must be alive");
            }
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/hecs", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_OP_COUNT)
            .map(|_| world.spawn(light_bundle()))
            .collect();

        b.iter(|| {
            for &entity in &entities {
                world
                    .insert_one(entity, Health(100.0))
                    .expect("benchmark entity must be alive and lack Health");
            }
            for &entity in &entities {
                world
                    .remove_one::<Health>(entity)
                    .expect("benchmark entity must be alive and contain Health");
            }
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (mut world, random_entities, churn_entities) = mixed_world();
    let mut move_query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
    let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::default();
    let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::default();
    let mut heavy_query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::default();
    let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    group.bench_function("frame/hecs", |b| {
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
    {
        let (world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::default();
        group.bench_function("movement/hecs", |b| {
            b.iter(|| {
                mixed_move_step(&world, &mut query);
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::default();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::default();
        group.bench_function("health/hecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    mixed_health_step(&world, &mut enemy_query, &mut ally_query);
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::default();
        group.bench_function("heavy/hecs", |b| {
            b.iter(|| {
                mixed_heavy_step(&world, &mut query);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, random_entities, _) = mixed_world();
        let mut query = PreparedQuery::<&PositionComponent>::default();
        let view = query.view_mut(&mut world);
        group.bench_function("random_access/hecs", |b| {
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
    }

    {
        let (mut world, _, churn_entities) = mixed_world();
        group.bench_function("structural_churn/hecs", |b| {
            b.iter(|| {
                mixed_churn_step(&mut world, &churn_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/hecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    mixed_spawn_step(&mut world, &mut spawned_entities);
                }
                black_box(&world);
            });
        });
    }
}

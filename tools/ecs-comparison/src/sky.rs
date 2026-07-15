use crate::common::*;
use crate::shared::sample_entities;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use sky_ecs::dynamic::{DynamicBundle, WorldDynamicExt};
use sky_ecs::{Bundle, EntityId, PreparedQuery, World};
use std::hint::black_box;

fn prepared_insert_world() -> World {
    // Sky caches bundle metadata process-wide. Make that reusable schema work
    // explicit in setup while leaving the World empty and without row capacity.
    let _ = <SuiteBundle as Bundle>::archetype();
    World::new()
}

fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..n).map(|_| suite_bundle()));
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($tag:ty) => {{
            for _ in 0..FRAGMENTED_ENTITIES_PER_VARIANT {
                world.spawn((<$tag>::default(), DataComponent(1.0)));
            }
        }};
    }

    add_variant!(A);
    add_variant!(B);
    add_variant!(C);
    add_variant!(D);
    add_variant!(E);
    add_variant!(F);
    add_variant!(G);
    add_variant!(H);
    add_variant!(I);
    add_variant!(J);
    add_variant!(K);
    add_variant!(L);
    add_variant!(M);
    add_variant!(N);
    add_variant!(O);
    add_variant!(P);
    add_variant!(Q);
    add_variant!(R);
    add_variant!(S);
    add_variant!(T);
    add_variant!(U);
    add_variant!(V);
    add_variant!(W);
    add_variant!(X);
    add_variant!(Y);
    add_variant!(Z);

    world
}

fn random_fragmented_world(component_count: usize, entity_count: usize) -> (World, usize) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let expected = random_fragment_match_count(&masks);
    let mut world = World::new();

    for mask in masks {
        let entity = world
            .spawn_dynamic(DynamicBundle::new())
            .expect("empty dynamic bundle should be valid");
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    assert!(world.insert(entity, $component(10.0)));
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

fn mixed_world() -> (World, Vec<EntityId>, Vec<EntityId>) {
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
    move_query.for_each(&mut *world, |(position, velocity)| {
        position.0 += velocity.0;
    });
}

fn mixed_health_step(
    world: &mut World,
    enemy_query: &mut PreparedQuery<(&mut Health, &Damage)>,
    ally_query: &mut PreparedQuery<(&mut Health, &Regen)>,
) {
    enemy_query.for_each(&mut *world, |(health, damage)| {
        health.0 -= damage.0;
    });

    ally_query.for_each(&mut *world, |(health, regen)| {
        health.0 += regen.0;
    });
}

fn mixed_heavy_step(
    world: &mut World,
    heavy_query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) {
    heavy_query.for_each(&mut *world, |(position, transform)| {
        let base = transform.0;
        let mut matrix = base;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = black_box(base)
                .invert()
                .expect("mixed-frame matrix should be invertible");
        }
        position.0 = matrix.transform_vector(position.0);
    });
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
        assert!(world.insert(entity, Health(100.0)));
    }
    for &entity in churn_entities {
        assert!(world.remove::<Health>(entity));
    }
}

fn mixed_spawn_step(world: &mut World, spawned_entities: &mut Vec<EntityId>) {
    spawned_entities.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned_entities.push(world.spawn(light_bundle()));
    }
    for &entity in spawned_entities.iter() {
        assert!(world.despawn(entity));
    }
}

fn run_mixed_frame(
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
    mixed_heavy_step(world, heavy_query);
    let checksum = mixed_random_step(world, random_entities);
    mixed_churn_step(world, churn_entities);
    mixed_spawn_step(world, spawned_entities);
    checksum
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
    group.bench_function("bulk_insert_10k/sky", |b| {
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                world.spawn_batch(bundles.iter().copied());
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/sky", |b| {
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
    assert_eq!(construction_world.entity_count(), 0);

    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    assert_eq!(world.entity_count(), CONTRACT_ENTITY_COUNT);
    let mut count = 0;
    let mut checksum = 0.0;
    PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new().for_each(
        &mut world,
        |(position, velocity)| {
            position.0 += velocity.0;
            count += 1;
            checksum += position.0.x;
        },
    );
    assert_eq!(count, CONTRACT_ENTITY_COUNT);
    assert_eq!(checksum, 256.0);

    let entity = world.spawn(light_bundle());
    assert!(world.get::<PositionComponent>(entity).is_some());
    assert!(world.insert(entity, Health(100.0)));
    assert!(world.get::<Health>(entity).is_some());
    assert!(world.remove::<Health>(entity));
    assert!(world.get::<Health>(entity).is_none());
    assert!(world.despawn(entity));
    assert!(!world.contains(entity));
    assert!(world.get::<PositionComponent>(entity).is_none());

    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.spawn(light_bundle()))
        .collect();
    let positions = random_world.accessor::<PositionComponent>();
    let random_checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
        add_position_checksum(
            checksum,
            positions
                .get(entity)
                .expect("contract entity must be readable through accessor"),
        )
    });
    assert_eq!(
        random_checksum,
        position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
    );

    let mut fragmented = fragmented_world();
    let mut fragmented_count = 0;
    PreparedQuery::<&DataComponent>::new().for_each(&mut fragmented, |_| fragmented_count += 1);
    assert_eq!(
        fragmented_count,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (mut random_fragmented, expected) =
            random_fragmented_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        let mut matched = 0;
        let mut values = 0.0;
        PreparedQuery::<(&A, &B, &C, &D)>::new().for_each(
            &mut random_fragmented,
            |(a, b, c, d)| {
                matched += 1;
                values += a.0 + b.0 + c.0 + d.0;
            },
        );
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * 40.0);
    }

    let base_count = world.entity_count();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    assert_eq!(world.entity_count(), base_count + ENTITY_OP_COUNT);
    for &entity in &entity_ops {
        assert!(world.despawn(entity));
    }
    assert_eq!(world.entity_count(), base_count);
    assert!(entity_ops.iter().all(|&entity| !world.contains(entity)));

    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.entity_count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    for &entity in &churn {
        assert!(mixed.insert(entity, Health(100.0)));
    }
    assert!(churn
        .iter()
        .all(|&entity| mixed.get::<Health>(entity).is_some()));
    for &entity in &churn {
        assert!(mixed.remove::<Health>(entity));
    }

    let random_checksum = run_mixed_frame(
        &mut mixed,
        &mut PreparedQuery::new(),
        &mut PreparedQuery::new(),
        &mut PreparedQuery::new(),
        &mut PreparedQuery::new(),
        &random,
        &churn,
        &mut spawned,
    );
    assert_ne!(random_checksum, 0);
    assert_eq!(mixed.entity_count(), expected);
    assert!(mixed.get::<Health>(churn[0]).is_none());
    assert!(spawned.iter().all(|&entity| !mixed.contains(entity)));

    let mut position_count = 0;
    let mut position_sum = 0.0;
    PreparedQuery::<&PositionComponent>::new().for_each(&mut mixed, |position| {
        position_count += 1;
        position_sum += position.0.x;
    });
    assert_eq!(
        position_count,
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
    );
    assert_approx_eq(position_sum, 18_500.0);

    let mut health_count = 0;
    let mut health_sum = 0.0;
    PreparedQuery::<&Health>::new().for_each(&mut mixed, |health| {
        health_count += 1;
        health_sum += health.0;
    });
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();

    group.bench_function("simple_10k/sky", |b| {
        b.iter(|| {
            query.for_each(&mut world, |(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();

    group.bench_function("simple_x32/sky", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                query.for_each(&mut world, |(pos, vel)| {
                    pos.0 += vel.0;
                });
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();

    group.bench_function("simple_100k/sky", |b| {
        b.iter(|| {
            query.for_each(&mut world, |(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let mut world = fragmented_world();
    let mut query = PreparedQuery::<&mut DataComponent>::new();

    group.bench_function("fragmented_26x400/sky", |b| {
        b.iter(|| {
            query.for_each(&mut world, |data| {
                data.0 = -data.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (mut world, expected) =
            random_fragmented_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
        let mut query = PreparedQuery::<(&A, &B, &C, &D)>::new();
        let mut initial_count = 0;
        query.for_each_with_entity(&mut world, |_, _| initial_count += 1);
        assert_eq!(initial_count, expected);

        group.bench_function(
            format!("random_{component_count}_components_4_terms/sky"),
            |b| {
                let mut checksum = 0_u64;
                b.iter(|| {
                    query.for_each_with_entity(&mut world, |entity, (a, b, c, d)| {
                        checksum = checksum
                            .wrapping_add(entity.index() as u64)
                            .wrapping_add((entity.generation() as u64) << 32)
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
    let mut world = heavy_world();
    let mut query = PreparedQuery::<(&mut PositionComponent, &mut TransformComponent)>::new();

    group.bench_function("heavy/sky", |b| {
        b.iter(|| {
            query.for_each(&mut world, |(position, transform)| {
                let base = transform.0;
                let mut matrix = base;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("base heavy matrix should be invertible");
                }
                position.0 = matrix.transform_vector(position.0);
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
        let mut world = World::new();
        let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
        let orders = deterministic_orders(&entities);
        let positions = world.accessor::<PositionComponent>();
        let mut order = 0;
        group.bench_function(format!("{name}/sky"), |b| {
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = positions
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
    group.bench_function("spawn_despawn_1k/sky", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(world.spawn(light_bundle()));
            }
            for &entity in &entities {
                assert!(world.despawn(entity));
            }
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/sky", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_OP_COUNT)
            .map(|_| world.spawn(light_bundle()))
            .collect();

        b.iter(|| {
            for &entity in &entities {
                assert!(world.insert(entity, Health(100.0)));
            }
            for &entity in &entities {
                assert!(world.remove::<Health>(entity));
            }
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (mut world, random_entities, churn_entities) = mixed_world();
    let mut move_query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
    let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::new();
    let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::new();
    let mut heavy_query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
    let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    group.bench_function("frame/sky", |b| {
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
        let (mut world, _, _) = mixed_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        group.bench_function("movement/sky", |b| {
            b.iter(|| {
                mixed_move_step(&mut world, &mut query);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut enemy_query = PreparedQuery::<(&mut Health, &Damage)>::new();
        let mut ally_query = PreparedQuery::<(&mut Health, &Regen)>::new();
        group.bench_function("health/sky", |b| {
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
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        group.bench_function("heavy/sky", |b| {
            b.iter(|| {
                mixed_heavy_step(&mut world, &mut query);
                black_box(&world);
            });
        });
    }

    {
        let (world, random_entities, _) = mixed_world();
        let positions = world.accessor::<PositionComponent>();
        group.bench_function("random_access/sky", |b| {
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
    }

    {
        let (mut world, _, churn_entities) = mixed_world();
        group.bench_function("structural_churn/sky", |b| {
            b.iter(|| {
                mixed_churn_step(&mut world, &churn_entities);
                black_box(&world);
            });
        });
    }

    {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/sky", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    mixed_spawn_step(&mut world, &mut spawned_entities);
                }
                black_box(&world);
            });
        });
    }
}

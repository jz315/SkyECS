use crate::common::*;
use crate::shared::sample_entities;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use shipyard::{EntityId, Get, IntoIter, View, ViewMut, World};
use std::hint::black_box;

fn prepared_insert_world() -> World {
    let world = World::new();
    // Mutable views lazily install empty component storages. Dropping the views
    // keeps the World at zero entities and does not reserve row capacity.
    drop(
        world
            .borrow::<(
                ViewMut<TransformComponent>,
                ViewMut<PositionComponent>,
                ViewMut<RotationComponent>,
                ViewMut<VelocityComponent>,
            )>()
            .expect("construction component storages should be available"),
    );
    world
}

fn world_with_entities(n: usize) -> World {
    let mut world = World::new();
    world.bulk_add_entity((0..n).map(|_| suite_bundle()));
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($world:ident; $($tag:ident),* $(,)?) => {
            $( $world.bulk_add_entity((0..FRAGMENTED_ENTITIES_PER_VARIANT).map(|_| ($tag(0.0), DataComponent(1.0)))); )*
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
        let entity = world.add_entity(());
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    world.add_component(entity, ($component(10.0),));
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
    world.bulk_add_entity((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

fn mixed_world() -> (World, Vec<EntityId>, Vec<EntityId>) {
    let mut world = World::new();
    let mut all_entities = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );
    let mut churn_entities = Vec::with_capacity(MIXED_FRAME_CHURN_COUNT);

    for _ in 0..MIXED_FRAME_MOVERS {
        let entity = world.add_entity(mixed_mover_bundle());
        if churn_entities.len() < MIXED_FRAME_CHURN_COUNT {
            churn_entities.push(entity);
        }
        all_entities.push(entity);
    }
    for _ in 0..MIXED_FRAME_ENEMIES {
        all_entities.push(world.add_entity(mixed_enemy_bundle()));
    }
    for _ in 0..MIXED_FRAME_ALLIES {
        all_entities.push(world.add_entity(mixed_ally_bundle()));
    }
    for _ in 0..MIXED_FRAME_HEAVY {
        all_entities.push(world.add_entity(mixed_heavy_bundle()));
    }

    let random_entities = sample_entities(&all_entities, MIXED_FRAME_RANDOM_COUNT);
    (world, random_entities, churn_entities)
}

fn mixed_move_step(world: &World) {
    let (mut positions, velocities) = world
        .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
        .unwrap();
    (&mut positions, &velocities)
        .iter()
        .for_each(|(position, velocity)| position.0 += velocity.0);
}

fn mixed_health_step(world: &World) {
    {
        let (mut healths, damage) = world.borrow::<(ViewMut<Health>, View<Damage>)>().unwrap();
        (&mut healths, &damage)
            .iter()
            .for_each(|(health, damage)| health.0 -= damage.0);
    }
    {
        let (mut healths, regen) = world.borrow::<(ViewMut<Health>, View<Regen>)>().unwrap();
        (&mut healths, &regen)
            .iter()
            .for_each(|(health, regen)| health.0 += regen.0);
    }
}

fn mixed_heavy_step(world: &World) {
    let (mut positions, transforms) = world
        .borrow::<(ViewMut<PositionComponent>, View<TransformComponent>)>()
        .unwrap();
    (&mut positions, &transforms)
        .iter()
        .for_each(|(position, transform)| {
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

fn mixed_random_step(world: &World, entities: &[EntityId]) -> u64 {
    let positions = world.borrow::<View<PositionComponent>>().unwrap();
    let mut checksum = 0_u64;
    for &entity in entities {
        let position = (&positions)
            .get(entity)
            .expect("sampled entity must contain PositionComponent");
        checksum = add_position_checksum(checksum, position);
    }
    checksum
}

fn mixed_churn_step(world: &mut World, entities: &[EntityId]) {
    for &entity in entities {
        world.add_component(entity, (Health(100.0),));
    }
    for &entity in entities {
        world.delete_component::<(Health,)>(entity);
    }
}

fn mixed_spawn_step(world: &mut World, spawned: &mut Vec<EntityId>) {
    spawned.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned.push(world.add_entity(light_bundle()));
    }
    for &entity in spawned.iter() {
        assert!(world.delete_entity(entity));
    }
}

pub fn validate_contract() {
    let construction_world = prepared_insert_world();
    assert_eq!(
        (&construction_world
            .borrow::<View<PositionComponent>>()
            .expect("prepared position storage should be readable"))
            .iter()
            .count(),
        0
    );

    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let mut count = 0;
    let mut checksum = 0.0;
    {
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        (&mut positions, &velocities)
            .iter()
            .for_each(|(position, velocity)| {
                position.0 += velocity.0;
                count += 1;
                checksum += position.0.x;
            });
    }
    assert_eq!(count, CONTRACT_ENTITY_COUNT);
    assert_eq!(checksum, 256.0);

    let entity = world.add_entity(light_bundle());
    assert!(world.is_entity_alive(entity));
    world.add_component(entity, (Health(100.0),));
    assert!(world.borrow::<View<Health>>().unwrap().get(entity).is_ok());
    world.delete_component::<(Health,)>(entity);
    assert!(world.borrow::<View<Health>>().unwrap().get(entity).is_err());
    assert!(world.delete_entity(entity));
    assert!(!world.is_entity_alive(entity));

    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.add_entity(light_bundle()))
        .collect();
    let positions = random_world.borrow::<View<PositionComponent>>().unwrap();
    let random_checksum = random_entities.iter().fold(0_u64, |checksum, &entity| {
        add_position_checksum(
            checksum,
            (&positions)
                .get(entity)
                .expect("contract entity must be readable through View"),
        )
    });
    assert_eq!(
        random_checksum,
        position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
    );

    let fragmented = fragmented_world();
    assert_eq!(
        fragmented
            .borrow::<View<DataComponent>>()
            .unwrap()
            .iter()
            .count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (random_fragmented, expected) =
            random_fragmented_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        let (a, b, c, d) = random_fragmented
            .borrow::<(View<A>, View<B>, View<C>, View<D>)>()
            .unwrap();
        let mut matched = 0;
        let mut values = 0.0;
        (&a, &b, &c, &d).iter().for_each(|(a, b, c, d)| {
            matched += 1;
            values += a.0 + b.0 + c.0 + d.0;
        });
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * 40.0);
    }

    let base_count = world
        .borrow::<View<PositionComponent>>()
        .unwrap()
        .iter()
        .count();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.add_entity(light_bundle()))
        .collect();
    assert_eq!(
        world
            .borrow::<View<PositionComponent>>()
            .unwrap()
            .iter()
            .count(),
        base_count + ENTITY_OP_COUNT
    );
    for &entity in &entity_ops {
        assert!(world.delete_entity(entity));
    }
    assert_eq!(
        world
            .borrow::<View<PositionComponent>>()
            .unwrap()
            .iter()
            .count(),
        base_count
    );
    assert!(entity_ops
        .iter()
        .all(|&entity| !world.is_entity_alive(entity)));

    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed
        .borrow::<View<PositionComponent>>()
        .unwrap()
        .iter()
        .count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    for &entity in &churn {
        mixed.add_component(entity, (Health(100.0),));
    }
    {
        let health = mixed.borrow::<View<Health>>().unwrap();
        assert!(churn.iter().all(|&entity| (&health).get(entity).is_ok()));
    }
    for &entity in &churn {
        mixed.delete_component::<(Health,)>(entity);
    }

    mixed_move_step(&mixed);
    mixed_health_step(&mixed);
    mixed_heavy_step(&mixed);
    let random_checksum = mixed_random_step(&mixed, &random);
    assert_ne!(random_checksum, 0);
    mixed_churn_step(&mut mixed, &churn);
    mixed_spawn_step(&mut mixed, &mut spawned);
    assert_eq!(
        mixed
            .borrow::<View<PositionComponent>>()
            .unwrap()
            .iter()
            .count(),
        expected
    );
    assert!(mixed
        .borrow::<View<Health>>()
        .unwrap()
        .get(churn[0])
        .is_err());
    assert!(spawned.iter().all(|&entity| !mixed.is_entity_alive(entity)));

    let positions = mixed.borrow::<View<PositionComponent>>().unwrap();
    let mut position_count = 0;
    let mut position_sum = 0.0;
    (&positions).iter().for_each(|position| {
        position_count += 1;
        position_sum += position.0.x;
    });
    assert_eq!(
        position_count,
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
    );
    assert_approx_eq(position_sum, 18_500.0);
    drop(positions);

    let health = mixed.borrow::<View<Health>>().unwrap();
    let mut health_count = 0;
    let mut health_sum = 0.0;
    (&health).iter().for_each(|value| {
        health_count += 1;
        health_sum += value.0;
    });
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
    group.bench_function("bulk_insert_10k/shipyard", |b| {
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                black_box(world.bulk_add_entity(bundles.iter().copied()));
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/shipyard", |b| {
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                for &bundle in &bundles {
                    world.add_entity(bundle);
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let (mut positions, velocities) = world
        .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
        .unwrap();

    group.bench_function("simple_10k/shipyard", |b| {
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let (mut positions, velocities) = world
        .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
        .unwrap();

    group.bench_function("simple_x32/shipyard", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                (&mut positions, &velocities)
                    .iter()
                    .for_each(|(position, velocity)| position.0 += velocity.0);
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let (mut positions, velocities) = world
        .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
        .unwrap();

    group.bench_function("simple_100k/shipyard", |b| {
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);
    let world = fragmented_world();
    let mut data = world.borrow::<ViewMut<DataComponent>>().unwrap();

    group.bench_function("fragmented_26x400/shipyard", |b| {
        b.iter(|| {
            (&mut data).iter().for_each(|data| data.0 = -data.0);
            black_box(&world);
        });
    });
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (world, expected) =
            random_fragmented_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
        let (a, b_view, c, d) = world
            .borrow::<(View<A>, View<B>, View<C>, View<D>)>()
            .unwrap();
        assert_eq!((&a, &b_view, &c, &d).iter().count(), expected);

        group.bench_function(
            format!("random_{component_count}_components_4_terms/shipyard"),
            |bencher| {
                let mut checksum = 0_u64;
                bencher.iter(|| {
                    (&a, &b_view, &c, &d)
                        .iter()
                        .with_id()
                        .for_each(|(entity, (a, b, c, d))| {
                            checksum = checksum
                                .wrapping_add(entity.inner())
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
    let (mut positions, mut transforms) = world
        .borrow::<(ViewMut<PositionComponent>, ViewMut<TransformComponent>)>()
        .unwrap();

    group.bench_function("heavy/shipyard", |b| {
        b.iter(|| {
            (&mut positions, &mut transforms)
                .iter()
                .for_each(|(position, transform)| {
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
        let entities: Vec<_> = (0..count)
            .map(|_| world.add_entity(light_bundle()))
            .collect();
        let orders = deterministic_orders(&entities);
        let positions = world.borrow::<View<PositionComponent>>().unwrap();
        let mut order = 0;
        group.bench_function(format!("{name}/shipyard"), |b| {
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = (&positions)
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
    group.bench_function("spawn_despawn_1k/shipyard", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(world.add_entity(light_bundle()));
            }
            for &entity in &entities {
                assert!(world.delete_entity(entity));
            }
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/shipyard", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_OP_COUNT)
            .map(|_| world.add_entity(light_bundle()))
            .collect();
        b.iter(|| {
            for &entity in &entities {
                world.add_component(entity, (Health(100.0),));
            }
            for &entity in &entities {
                world.delete_component::<(Health,)>(entity);
            }
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (mut world, random_entities, churn_entities) = mixed_world();
    let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    group.bench_function("frame/shipyard", |b| {
        b.iter(|| {
            mixed_move_step(&world);
            mixed_health_step(&world);
            mixed_heavy_step(&world);
            let checksum = mixed_random_step(&world, &random_entities);
            mixed_churn_step(&mut world, &churn_entities);
            mixed_spawn_step(&mut world, &mut spawned_entities);
            black_box(checksum);
            black_box(&world);
        });
    });
}

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    {
        let (world, _, _) = mixed_world();
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        group.bench_function("movement/shipyard", |b| {
            b.iter(|| {
                (&mut positions, &velocities)
                    .iter()
                    .for_each(|(position, velocity)| position.0 += velocity.0);
                black_box(&world);
            });
        });
    }
    {
        let (world, _, _) = mixed_world();
        let (mut healths, damage, regen) = world
            .borrow::<(ViewMut<Health>, View<Damage>, View<Regen>)>()
            .unwrap();
        group.bench_function("health/shipyard", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    (&mut healths, &damage)
                        .iter()
                        .for_each(|(health, damage)| health.0 -= damage.0);
                    (&mut healths, &regen)
                        .iter()
                        .for_each(|(health, regen)| health.0 += regen.0);
                }
                black_box(&world);
            });
        });
    }
    {
        let (world, _, _) = mixed_world();
        let (mut positions, transforms) = world
            .borrow::<(ViewMut<PositionComponent>, View<TransformComponent>)>()
            .unwrap();
        group.bench_function("heavy/shipyard", |b| {
            b.iter(|| {
                (&mut positions, &transforms)
                    .iter()
                    .for_each(|(position, transform)| {
                        let base = transform.0;
                        let mut matrix = base;
                        for _ in 0..MIXED_FRAME_INVERT_COUNT {
                            matrix = black_box(base)
                                .invert()
                                .expect("mixed-frame matrix should be invertible");
                        }
                        position.0 = matrix.transform_vector(position.0);
                    });
                black_box(&world);
            });
        });
    }
    {
        let (world, random_entities, _) = mixed_world();
        let positions = world.borrow::<View<PositionComponent>>().unwrap();
        group.bench_function("random_access/shipyard", |b| {
            b.iter(|| {
                let mut checksum = 0_u64;
                for &entity in &random_entities {
                    let position = (&positions)
                        .get(entity)
                        .expect("sampled entity must contain PositionComponent");
                    checksum = add_position_checksum(checksum, position);
                }
                black_box(checksum);
                black_box(&world);
            });
        });
    }
    {
        let (mut world, _, churn_entities) = mixed_world();
        group.bench_function("structural_churn/shipyard", |b| {
            b.iter(|| {
                mixed_churn_step(&mut world, &churn_entities);
                black_box(&world);
            });
        });
    }
    {
        let (mut world, _, _) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/shipyard", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    mixed_spawn_step(&mut world, &mut spawned_entities);
                }
                black_box(&world);
            });
        });
    }
}

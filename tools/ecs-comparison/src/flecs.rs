use crate::common::*;
use crate::shared::sample_entities;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use flecs_ecs::core::{Entity as FlecsEntity, EntityViewGet, IdOperations, QueryAPI, World};
use std::hint::black_box;

// ---------------------------------------------------------------------------
// World builders
// ---------------------------------------------------------------------------

fn world_with_entities(n: usize) -> World {
    let world = World::new();
    for _ in 0..n {
        let (t, p, r, v) = suite_bundle();
        world.entity().set(t).set(p).set(r).set(v);
    }
    world
}

fn fragmented_world() -> World {
    let world = World::new();

    macro_rules! add_variant {
        ($tag:ty) => {
            for _ in 0..FRAGMENTED_ENTITIES_PER_VARIANT {
                world
                    .entity()
                    .set(<$tag>::default())
                    .set(DataComponent(1.0));
            }
        };
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

fn heavy_world() -> World {
    let world = World::new();
    for _ in 0..HEAVY_ENTITY_COUNT {
        let (t, p, r, v) = heavy_bundle();
        world.entity().set(t).set(p).set(r).set(v);
    }
    world
}

/// Returns (world, random_entity_ids, churn_entity_ids).
/// Entity IDs stored as `FlecsEntity` (`Copy` u64 wrapper) to outlive borrows.
fn mixed_world() -> (World, Vec<FlecsEntity>, Vec<FlecsEntity>) {
    let world = World::new();
    let mut all_ids = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );
    let mut churn_ids = Vec::with_capacity(MIXED_FRAME_CHURN_COUNT);

    for _ in 0..MIXED_FRAME_MOVERS {
        let (p, v) = mixed_mover_bundle();
        let e = world.entity().set(p).set(v);
        let id = e.id();
        if churn_ids.len() < MIXED_FRAME_CHURN_COUNT {
            churn_ids.push(id);
        }
        all_ids.push(id);
    }

    // IsEnemy/IsAlly are zero-sized tags — use `add(component_id)` instead of `set()`.
    let enemy_tag = world.component_id::<IsEnemy>();
    let ally_tag = world.component_id::<IsAlly>();

    for _ in 0..MIXED_FRAME_ENEMIES {
        let (p, v, h, d, _tag) = mixed_enemy_bundle();
        let e = world.entity().set(p).set(v).set(h).set(d).add(enemy_tag);
        all_ids.push(e.id());
    }

    for _ in 0..MIXED_FRAME_ALLIES {
        let (p, v, h, r, _tag) = mixed_ally_bundle();
        let e = world.entity().set(p).set(v).set(h).set(r).add(ally_tag);
        all_ids.push(e.id());
    }

    for _ in 0..MIXED_FRAME_HEAVY {
        let (t, p, v) = mixed_heavy_bundle();
        let e = world.entity().set(t).set(p).set(v);
        all_ids.push(e.id());
    }

    let random_ids = sample_entities(&all_ids, MIXED_FRAME_RANDOM_COUNT);
    (world, random_ids, churn_ids)
}

// ---------------------------------------------------------------------------
// Insert benchmarks
// ---------------------------------------------------------------------------

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    let transforms = vec![suite_transform(); SIMPLE_ENTITY_COUNT];
    let positions = vec![suite_position(); SIMPLE_ENTITY_COUNT];
    let rotations = vec![suite_rotation(); SIMPLE_ENTITY_COUNT];
    let velocities = vec![suite_velocity(); SIMPLE_ENTITY_COUNT];

    group.bench_function("bulk_insert_10k/flecs", |b| {
        b.iter_batched_ref(
            World::new,
            |world| {
                let entities = world
                    .entity_bulk(SIMPLE_ENTITY_COUNT as u32)
                    .set(&transforms)
                    .set(&positions)
                    .set(&rotations)
                    .set(&velocities)
                    .build();
                black_box(entities);
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/flecs", |b| {
        b.iter_batched_ref(
            World::new,
            |world| {
                for _ in 0..SIMPLE_ENTITY_COUNT {
                    let (t, p, r, v) = suite_bundle();
                    world.entity().set(t).set(p).set(r).set(v);
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn validate_contract() {
    let bulk_world = World::new();
    let transforms = vec![suite_transform(); 16];
    let positions = vec![suite_position(); 16];
    let rotations = vec![suite_rotation(); 16];
    let velocities = vec![suite_velocity(); 16];
    let bulk_entities = bulk_world
        .entity_bulk(16)
        .set(&transforms)
        .set(&positions)
        .set(&rotations)
        .set(&velocities)
        .build();
    assert_eq!(bulk_entities.len(), 16);
    assert_eq!(
        bulk_world.new_query::<&PositionComponent>().count() as usize,
        16
    );

    let world = world_with_entities(128);
    let query = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();
    let mut count = 0;
    let mut checksum = 0.0;
    query.each(|(position, velocity)| {
        position.0 += velocity.0;
        count += 1;
        checksum += position.0.x;
    });
    assert_eq!(count, 128);
    assert_eq!(checksum, 256.0);

    let (position, velocity) = light_bundle();
    let entity = world.entity().set(position).set(velocity);
    let id = entity.id();
    let health_id = world.component_id::<Health>();
    assert!(entity.is_alive());
    entity.set(Health(100.0));
    assert!(entity.has(health_id));
    entity.remove(health_id);
    assert!(!entity.has(health_id));
    entity.destruct();
    assert!(!world.is_alive(id));

    let fragmented = fragmented_world();
    assert_eq!(
        fragmented.new_query::<&DataComponent>().count() as usize,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );

    let (mixed, random, churn) = mixed_world();
    let expected = mixed.new_query::<&PositionComponent>().count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    let move_query = mixed.new_query::<(&mut PositionComponent, &VelocityComponent)>();
    let enemy_query = mixed.new_query::<(&mut Health, &Damage)>();
    let ally_query = mixed.new_query::<(&mut Health, &Regen)>();
    let heavy_query = mixed.new_query::<(&mut PositionComponent, &TransformComponent)>();
    move_query.each(|(position, velocity)| position.0 += velocity.0);
    enemy_query.each(|(health, damage)| health.0 -= damage.0);
    ally_query.each(|(health, regen)| health.0 += regen.0);
    heavy_query.each(|_| {});
    for &random_id in &random {
        mixed
            .entity_from_id(random_id)
            .get::<&PositionComponent>(|_| {});
    }
    let health_id = mixed.component_id::<Health>();
    for &churn_id in &churn {
        mixed.entity_from_id(churn_id).set(Health(100.0));
        mixed.entity_from_id(churn_id).remove(health_id);
    }
    mixed_spawn_step_for_validation(&mixed, &mut spawned);
    assert_eq!(mixed.new_query::<&PositionComponent>().count(), expected);
    assert!(!mixed.entity_from_id(churn[0]).has(health_id));

    let mut deferred_ids = Vec::with_capacity(ENTITY_OP_COUNT);
    spawn_despawn_deferred(&mixed, &mut deferred_ids);
    assert_eq!(mixed.new_query::<&PositionComponent>().count(), expected);
    assert!(deferred_ids.iter().all(|&entity| !mixed.is_alive(entity)));
}

fn mixed_spawn_step_for_validation(world: &World, spawned: &mut Vec<FlecsEntity>) {
    spawned.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        let (position, velocity) = light_bundle();
        spawned.push(world.entity().set(position).set(velocity).id());
    }
    for &entity in spawned.iter() {
        world.entity_from_id(entity).destruct();
    }
}

fn spawn_despawn_direct(world: &World, ids: &mut Vec<FlecsEntity>) {
    ids.clear();
    for _ in 0..ENTITY_OP_COUNT {
        let (position, velocity) = light_bundle();
        ids.push(world.entity().set(position).set(velocity).id());
    }
    for &id in ids.iter() {
        world.entity_from_id(id).destruct();
    }
}

fn spawn_despawn_deferred(world: &World, ids: &mut Vec<FlecsEntity>) {
    ids.clear();
    world.defer_begin();
    for _ in 0..ENTITY_OP_COUNT {
        let (position, velocity) = light_bundle();
        ids.push(world.entity().set(position).set(velocity).id());
    }
    for &id in ids.iter() {
        world.entity_from_id(id).destruct();
    }
    world.defer_end();
}

// ---------------------------------------------------------------------------
// Iteration benchmarks  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    // The reusable query object stays outside the timed loop. Flecs' uncached
    // matching mode is faster for this workload than maintaining a table cache.
    let query = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_10k/flecs", |b| {
        b.iter(|| {
            query.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let query = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_x32/flecs", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                query.each(|(pos, vel)| {
                    pos.0 += vel.0;
                });
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let query = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();

    group.bench_function("simple_100k/flecs", |b| {
        b.iter(|| {
            query.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let world = fragmented_world();
    let query = world.new_query::<&mut DataComponent>();

    group.bench_function("fragmented_26x400/flecs", |b| {
        b.iter(|| {
            query.each(|data| {
                data.0 *= 2.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = heavy_world();
    let query = world.new_query::<(&mut PositionComponent, &mut TransformComponent)>();

    group.bench_function("heavy/flecs", |b| {
        b.iter(|| {
            query.each(|(position, transform)| {
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

// ---------------------------------------------------------------------------
// Random access benchmark
// ---------------------------------------------------------------------------

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
        ("cold_1m", COLD_RANDOM_ENTITY_COUNT),
    ] {
        let world = World::new();
        let ids: Vec<FlecsEntity> = (0..count)
            .map(|_| {
                let (position, velocity) = light_bundle();
                world.entity().set(position).set(velocity).id()
            })
            .collect();
        let orders = deterministic_orders(&ids);
        let mut order = 0;
        group.bench_function(format!("{name}/flecs"), |b| {
            b.iter(|| {
                let ids = &orders[order % orders.len()];
                order += 1;
                for &id in ids {
                    world
                        .entity_from_id(id)
                        .get::<&PositionComponent>(|position| {
                            black_box(position);
                        });
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Entity operations benchmark
// ---------------------------------------------------------------------------

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/flecs", |b| {
        let world = World::new();
        let mut ids = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            spawn_despawn_direct(&world, &mut ids);
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/flecs", |b| {
        let world = World::new();
        let ids: Vec<FlecsEntity> = (0..ENTITY_OP_COUNT)
            .map(|_| {
                let (p, v) = light_bundle();
                world.entity().set(p).set(v).id()
            })
            .collect();
        let health_id = world.component_id::<Health>();

        b.iter(|| {
            for &id in &ids {
                world.entity_from_id(id).set(Health(100.0));
            }
            for &id in &ids {
                world.entity_from_id(id).remove(health_id);
            }
            black_box(&world);
        });
    });
}

pub fn bench_spawn_despawn_modes(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("direct_1k", |b| {
        let world = World::new();
        let mut ids = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            spawn_despawn_direct(&world, &mut ids);
            black_box(&world);
        });
    });

    group.bench_function("deferred_1k", |b| {
        let world = World::new();
        let mut ids = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            spawn_despawn_deferred(&world, &mut ids);
            black_box(&world);
        });
    });
}

// ---------------------------------------------------------------------------
// Mixed frame benchmark  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (world, random_ids, churn_ids) = mixed_world();
    let mut spawned_ids = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    // Keep the reusable query objects outside the timed loop. Uncached matching
    // avoids cache-maintenance overhead during this structural workload.
    let move_q = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();
    let enemy_q = world.new_query::<(&mut Health, &Damage)>();
    let ally_q = world.new_query::<(&mut Health, &Regen)>();
    let heavy_q = world.new_query::<(&mut PositionComponent, &TransformComponent)>();
    let health_id = world.component_id::<Health>();

    group.bench_function("frame/flecs", |b| {
        b.iter(|| {
            // Movement
            move_q.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            // Health
            enemy_q.each(|(health, damage)| {
                health.0 -= damage.0;
            });
            ally_q.each(|(health, regen)| {
                health.0 += regen.0;
            });
            // Heavy compute
            heavy_q.each(|(position, transform)| {
                let base = transform.0;
                let mut matrix = base;
                for _ in 0..MIXED_FRAME_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("mixed-frame matrix should be invertible");
                }
                position.0 = matrix.transform_vector(position.0);
            });
            // Random access
            for &id in &random_ids {
                world.entity_from_id(id).get::<&PositionComponent>(|p| {
                    black_box(p);
                });
            }
            // Structural churn
            for &id in &churn_ids {
                world.entity_from_id(id).set(Health(100.0));
            }
            for &id in &churn_ids {
                world.entity_from_id(id).remove(health_id);
            }
            // Spawn/despawn
            spawned_ids.clear();
            for _ in 0..MIXED_FRAME_SPAWN_COUNT {
                let (p, v) = light_bundle();
                spawned_ids.push(world.entity().set(p).set(v).id());
            }
            for &id in spawned_ids.iter() {
                world.entity_from_id(id).destruct();
            }

            black_box(&world);
        });
    });
}

// ---------------------------------------------------------------------------
// Mixed frame phases benchmark  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    {
        let (world, _, _) = mixed_world();
        let query = world.new_query::<(&mut PositionComponent, &VelocityComponent)>();
        group.bench_function("movement/flecs", |b| {
            b.iter(|| {
                query.each(|(pos, vel)| {
                    pos.0 += vel.0;
                });
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let enemy_q = world.new_query::<(&mut Health, &Damage)>();
        let ally_q = world.new_query::<(&mut Health, &Regen)>();
        group.bench_function("health/flecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    enemy_q.each(|(health, damage)| {
                        health.0 -= damage.0;
                    });
                    ally_q.each(|(health, regen)| {
                        health.0 += regen.0;
                    });
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let query = world.new_query::<(&mut PositionComponent, &TransformComponent)>();
        group.bench_function("heavy/flecs", |b| {
            b.iter(|| {
                query.each(|(position, transform)| {
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
        let (world, random_ids, _) = mixed_world();
        group.bench_function("random_access/flecs", |b| {
            b.iter(|| {
                for &id in &random_ids {
                    world.entity_from_id(id).get::<&PositionComponent>(|p| {
                        black_box(p);
                    });
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, churn_ids) = mixed_world();
        let health_id = world.component_id::<Health>();
        group.bench_function("structural_churn/flecs", |b| {
            b.iter(|| {
                for &id in &churn_ids {
                    world.entity_from_id(id).set(Health(100.0));
                }
                for &id in &churn_ids {
                    world.entity_from_id(id).remove(health_id);
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let mut spawned_ids = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/flecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    spawned_ids.clear();
                    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
                        let (p, v) = light_bundle();
                        spawned_ids.push(world.entity().set(p).set(v).id());
                    }
                    for &id in spawned_ids.iter() {
                        world.entity_from_id(id).destruct();
                    }
                }
                black_box(&world);
            });
        });
    }
}

use super::*;

pub(super) fn mixed_world() -> (World, Vec<EntityId>, Vec<EntityId>) {
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

pub(super) fn mixed_move_step(world: &World) {
    let (mut positions, velocities) = world
        .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
        .unwrap();
    (&mut positions, &velocities)
        .iter()
        .for_each(|(position, velocity)| position.0 += velocity.0);
}

pub(super) fn mixed_health_step(world: &World) {
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

pub(super) fn mixed_heavy_step(world: &World) {
    let (mut positions, transforms) = world
        .borrow::<(ViewMut<PositionComponent>, View<TransformComponent>)>()
        .unwrap();
    (&mut positions, &transforms)
        .iter()
        .for_each(|(position, transform)| {
            let mut matrix = transform.0;
            for _ in 0..MIXED_FRAME_INVERT_COUNT {
                matrix = matrix
                    .invert()
                    .expect("mixed-frame matrix should remain invertible");
            }
            position.0 = matrix.transform_vector(position.0);
        });
}

pub(super) fn mixed_random_step(world: &World, entities: &[EntityId]) -> u64 {
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

pub(super) fn mixed_churn_step(world: &mut World, entities: &[EntityId]) {
    for &entity in entities {
        world.add_component(entity, (Health(100.0),));
    }
    for &entity in entities {
        world.delete_component::<(Health,)>(entity);
    }
}

pub(super) fn mixed_spawn_step(world: &mut World, spawned: &mut Vec<EntityId>) {
    spawned.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned.push(world.add_entity(light_bundle()));
    }
    for &entity in spawned.iter() {
        let removed = world.delete_entity(entity);
        debug_assert!(removed);
    }
}
pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/shipyard", |b| {
        let (mut world, random_entities, churn_entities) = mixed_world();
        let mut spawned_entities = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
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
    group.bench_function("movement/shipyard", |b| {
        let (world, _, _) = mixed_world();
        let (mut positions, velocities) = world
            .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
            black_box(&world);
        });
    });

    group.bench_function("health/shipyard", |b| {
        let (world, _, _) = mixed_world();
        let (mut healths, damage, regen) = world
            .borrow::<(ViewMut<Health>, View<Damage>, View<Regen>)>()
            .unwrap();
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

    group.bench_function("heavy/shipyard", |b| {
        let (world, _, _) = mixed_world();
        let (mut positions, transforms) = world
            .borrow::<(ViewMut<PositionComponent>, View<TransformComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &transforms)
                .iter()
                .for_each(|(position, transform)| {
                    let mut matrix = transform.0;
                    for _ in 0..MIXED_FRAME_INVERT_COUNT {
                        matrix = matrix
                            .invert()
                            .expect("mixed-frame matrix should remain invertible");
                    }
                    position.0 = matrix.transform_vector(position.0);
                });
            black_box(&world);
        });
    });

    group.bench_function("random_access/shipyard", |b| {
        let (world, random_entities, _) = mixed_world();
        let positions = world.borrow::<View<PositionComponent>>().unwrap();
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

    group.bench_function("structural_churn/shipyard", |b| {
        let (mut world, _, churn_entities) = mixed_world();
        b.iter(|| {
            mixed_churn_step(&mut world, &churn_entities);
            black_box(&world);
        });
    });

    group.bench_function("spawn_despawn/shipyard", |b| {
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

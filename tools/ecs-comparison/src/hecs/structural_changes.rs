use super::*;

pub(super) fn despawn_entities(
    world: &mut World,
    entities: &[HecsEntity],
    deletion_order: &[usize],
) {
    for &index in deletion_order {
        let entity = entities[index];
        let removed = world.despawn(entity);
        debug_assert!(removed.is_ok());
    }
}

pub(super) fn add_remove_health(
    world: &mut World,
    entities: &[HecsEntity],
    add_order: &[usize],
    remove_order: &[usize],
) {
    for &index in add_order {
        let entity = entities[index];
        let inserted = world.insert_one(entity, Health(100.0));
        debug_assert!(inserted.is_ok());
    }
    for &index in remove_order {
        let entity = entities[index];
        let removed = world.remove_one::<Health>(entity);
        debug_assert!(removed.is_ok());
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/hecs", |b| {
        let mut world = World::new();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        let deletion_order = entity_deletion_order(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(world.spawn(light_bundle()));
            }
            despawn_entities(&mut world, &entities, &deletion_order);
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/hecs", |b| {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_OP_COUNT)
            .map(|_| world.spawn(light_bundle()))
            .collect();
        let (add_order, remove_order) = component_change_orders(ENTITY_OP_COUNT);

        b.iter(|| {
            add_remove_health(&mut world, &entities, &add_order, &remove_order);
            black_box(&world);
        });
    });
}

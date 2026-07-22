use super::*;

pub(super) fn spawn_light_batch(world: &mut World, count: usize) -> Vec<Entity> {
    world.spawn_batch(LIGHT_MASK, count, |table, index| {
        let (position, velocity) = light_bundle();
        table.position[index] = position;
        table.velocity[index] = velocity;
    })
}

pub(super) fn spawn_random_access_batch(world: &mut World, count: usize) -> Vec<Entity> {
    world.spawn_batch(LIGHT_MASK, count, |table, index| {
        let (position, velocity) = random_access_bundle(index);
        table.position[index] = position;
        table.velocity[index] = velocity;
    })
}

pub(super) fn spawn_light_one(world: &mut World) -> Entity {
    spawn_light_batch(world, 1)
        .pop()
        .expect("FreeCS should return the spawned entity")
}

pub(super) fn despawn_entities(world: &mut World, entities: &[Entity], deletion_order: &[usize]) {
    for &index in deletion_order {
        let entity = entities[index];
        let despawned = world.despawn_entities(std::slice::from_ref(&entity));
        debug_assert_eq!(despawned.as_slice(), [entity]);
    }
}

pub(super) fn add_remove_health(
    world: &mut World,
    entities: &[Entity],
    add_order: &[usize],
    remove_order: &[usize],
) {
    for &index in add_order {
        let entity = entities[index];
        world.set_health(entity, Health(100.0));
    }
    for &index in remove_order {
        let entity = entities[index];
        let removed = world.remove_health(entity);
        debug_assert!(removed);
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/freecs", |b| {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(ENTITY_OP_COUNT);
        let deletion_order = entity_deletion_order(ENTITY_OP_COUNT);
        b.iter(|| {
            entities.clear();
            for _ in 0..ENTITY_OP_COUNT {
                entities.push(spawn_light_one(&mut world));
            }
            despawn_entities(&mut world, &entities, &deletion_order);
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/freecs", |b| {
        let mut world = World::default();
        let entities = spawn_light_batch(&mut world, ENTITY_OP_COUNT);
        let (add_order, remove_order) = component_change_orders(ENTITY_OP_COUNT);

        b.iter(|| {
            add_remove_health(&mut world, &entities, &add_order, &remove_order);
            black_box(&world);
        });
    });
}

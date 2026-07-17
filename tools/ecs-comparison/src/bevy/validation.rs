use super::dense_iteration::world_with_entities;
use super::entity_insertion::prepared_insert_world;
use super::fragmented_iteration::fragmented_world;
use super::mixed_frame::{mixed_world, run_mixed_frame};
use super::random_fragmented_iteration::{
    random_fragmented_component_world, random_fragmented_tag_world,
};
use super::structural_changes::{add_remove_health, despawn_entities};
use crate::common::{
    add_position_checksum, add_random_fragment_checksum, add_random_fragment_component_1_checksum,
    add_random_fragment_component_8_checksum, assert_approx_eq, assert_suite_bundles_match,
    component_change_orders, deterministic_orders, distinct_suite_bundles, entity_deletion_order,
    expected_random_fragment_checksum, light_bundle, position_checksum_value,
    random_fragment_match_count, random_fragment_transition_shapes, Damage, DataComponent, Health,
    PositionComponent, Regen, RotationComponent, TagA, TagB, TagC, TagD, TagE, TagF, TagG, TagH,
    TransformComponent, VelocityComponent, A, B, C, CONTRACT_ENTITY_COUNT,
    CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT, D, E, ENTITY_OP_COUNT, F,
    FRAGMENTED_ENTITIES_PER_VARIANT, FRAGMENTED_VARIANT_COUNT, G, H, MIXED_FRAME_ALLIES,
    MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_MOVERS, MIXED_FRAME_SPAWN_COUNT,
    RANDOM_FRAGMENT_WORKLOADS,
};
use bevy_ecs::{entity::Entity as BevyEntity, world::World};

pub fn validate_contract() {
    validate_construction();
    validate_dense_iteration();
    validate_entity_lifecycle();
    validate_random_access();
    validate_fragmented_iteration();
    validate_random_fragmented_iteration();
    validate_structural_changes();
    validate_mixed_frame();
}

fn validate_construction() {
    let bootstrap_archetypes = World::new().archetypes().len();
    let construction_inputs = distinct_suite_bundles(8);
    for bulk in [true, false] {
        let mut construction_world = prepared_insert_world();
        assert_eq!(construction_world.archetypes().len(), bootstrap_archetypes);
        if bulk {
            construction_world.spawn_batch(construction_inputs.iter().copied());
        } else {
            for &bundle in &construction_inputs {
                construction_world.spawn(bundle);
            }
        }
        let mut query = construction_world.query::<(
            &TransformComponent,
            &PositionComponent,
            &RotationComponent,
            &VelocityComponent,
        )>();
        let mut actual = query
            .iter(&construction_world)
            .map(|(transform, position, rotation, velocity)| {
                (*transform, *position, *rotation, *velocity)
            })
            .collect::<Vec<_>>();
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
}

fn validate_dense_iteration() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    assert_eq!(
        world.query::<&PositionComponent>().iter(&world).count(),
        CONTRACT_ENTITY_COUNT
    );
    let mut count = 0;
    let mut checksum = 0.0;
    for (mut position, velocity) in world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .iter_mut(&mut world)
    {
        position.0 += velocity.0;
        count += 1;
        checksum += position.0.x;
    }
    assert_eq!(count, CONTRACT_ENTITY_COUNT);
    assert_eq!(checksum, 256.0);
}

fn validate_entity_lifecycle() {
    let mut world = world_with_entities(0);
    let entity = world.spawn(light_bundle()).id();
    assert!(world.get::<PositionComponent>(entity).is_some());
    world.entity_mut(entity).insert(Health(100.0));
    assert!(world.get::<Health>(entity).is_some());
    world.entity_mut(entity).remove::<Health>();
    assert!(world.get::<Health>(entity).is_none());
    assert!(world.despawn(entity));
    assert!(!world.entities().contains(entity));
    assert!(world.get::<PositionComponent>(entity).is_none());
}

fn validate_random_access() {
    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.spawn(light_bundle()).id())
        .collect();
    let random_orders = deterministic_orders(&random_entities);
    let random_query = random_world.query::<&PositionComponent>();
    for order in random_orders {
        let random_checksum = order.iter().fold(0_u64, |checksum, &entity| {
            add_position_checksum(
                checksum,
                random_query
                    .get_manual(&random_world, entity)
                    .expect("contract entity must be readable through QueryState"),
            )
        });
        assert_eq!(
            random_checksum,
            position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
        );
    }
}

fn validate_fragmented_iteration() {
    let mut fragmented = fragmented_world();
    for mut data in fragmented
        .query::<&mut DataComponent>()
        .iter_mut(&mut fragmented)
    {
        data.0 = -data.0;
    }
    assert_eq!(
        fragmented
            .query::<&DataComponent>()
            .iter(&fragmented)
            .filter(|data| data.0 == -1.0)
            .count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
}

fn validate_random_fragmented_iteration() {
    let bootstrap_archetypes = World::new().archetypes().len();
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        let (mut random_fragmented, masks) = random_fragmented_component_world(
            component_count,
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT,
        );
        assert_eq!(
            random_fragmented.archetypes().len(),
            bootstrap_archetypes - 1
                + random_fragment_transition_shapes(&masks, component_count).len()
        );
        let expected = random_fragment_match_count(&masks, term_count);
        let (matched, values, checksum) = match term_count {
            1 => {
                let mut query = random_fragmented.query::<(BevyEntity, &A)>();
                let mut result = (0, 0.0, 0_u64);
                for (entity, a) in query.iter(&random_fragmented) {
                    result.0 += 1;
                    result.1 += a.0;
                    result.2 =
                        add_random_fragment_component_1_checksum(result.2, entity.to_bits(), a.0);
                }
                result
            }
            4 => {
                let mut query = random_fragmented.query::<(BevyEntity, &A, &B, &C, &D)>();
                let mut result = (0, 0.0, 0_u64);
                for (entity, a, b, c, d) in query.iter(&random_fragmented) {
                    result.0 += 1;
                    result.1 += a.0 + b.0 + c.0 + d.0;
                    result.2 = add_random_fragment_checksum(
                        result.2,
                        entity.to_bits(),
                        a.0,
                        b.0,
                        c.0,
                        d.0,
                    );
                }
                result
            }
            8 => {
                let mut query =
                    random_fragmented.query::<(BevyEntity, &A, &B, &C, &D, &E, &F, &G, &H)>();
                let mut result = (0, 0.0, 0_u64);
                for (entity, a, b, c, d, e, f, g, h) in query.iter(&random_fragmented) {
                    result.0 += 1;
                    result.1 += a.0 + b.0 + c.0 + d.0 + e.0 + f.0 + g.0 + h.0;
                    result.2 = add_random_fragment_component_8_checksum(
                        result.2,
                        entity.to_bits(),
                        a.0,
                        b.0,
                        c.0,
                        d.0,
                        e.0,
                        f.0,
                        g.0,
                        h.0,
                    );
                }
                result
            }
            _ => unreachable!(),
        };
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * term_count as f32 * 10.0);
        let mut entities = random_fragmented
            .iter_entities()
            .map(|entity| (entity.id().index(), entity.id().to_bits()))
            .collect::<Vec<_>>();
        entities.sort_unstable_by_key(|(index, _)| *index);
        // Bevy 0.19 keeps one framework-owned bootstrap entity in a new World.
        // The workload entities are the subsequent, monotonically allocated ids.
        assert_eq!(entities.len(), masks.len() + 1);
        let entity_keys = entities
            .into_iter()
            .skip(1)
            .map(|(_, entity_key)| entity_key)
            .collect::<Vec<_>>();
        assert_eq!(
            checksum,
            expected_random_fragment_checksum(&entity_keys, &masks, term_count)
        );

        let (mut random_tags, masks) =
            random_fragmented_tag_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        assert_eq!(
            random_tags.archetypes().len(),
            bootstrap_archetypes - 1
                + random_fragment_transition_shapes(&masks, component_count).len()
        );
        let tag_matches = match term_count {
            1 => random_tags.query::<&TagA>().iter(&random_tags).count(),
            4 => random_tags
                .query::<(&TagA, &TagB, &TagC, &TagD)>()
                .iter(&random_tags)
                .count(),
            8 => random_tags
                .query::<(&TagA, &TagB, &TagC, &TagD, &TagE, &TagF, &TagG, &TagH)>()
                .iter(&random_tags)
                .count(),
            _ => unreachable!(),
        };
        assert_eq!(tag_matches, random_fragment_match_count(&masks, term_count));
    }
}

fn validate_structural_changes() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let base_count = world.query::<&PositionComponent>().iter(&world).count();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.spawn(light_bundle()).id())
        .collect();
    assert_eq!(
        world.query::<&PositionComponent>().iter(&world).count(),
        base_count + ENTITY_OP_COUNT
    );
    let (add_order, remove_order) = component_change_orders(entity_ops.len());
    add_remove_health(&mut world, &entity_ops, &add_order, &remove_order);
    assert!(entity_ops
        .iter()
        .all(|&entity| world.get::<Health>(entity).is_none()));
    let deletion_order = entity_deletion_order(entity_ops.len());
    despawn_entities(&mut world, &entity_ops, &deletion_order);
    assert_eq!(
        world.query::<&PositionComponent>().iter(&world).count(),
        base_count
    );
    assert!(entity_ops
        .iter()
        .all(|&entity| !world.entities().contains(entity)));
}

fn validate_mixed_frame() {
    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.query::<&PositionComponent>().iter(&mixed).count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    let mut move_query = mixed.query::<(&mut PositionComponent, &VelocityComponent)>();
    let mut enemy_query = mixed.query::<(&mut Health, &Damage)>();
    let mut ally_query = mixed.query::<(&mut Health, &Regen)>();
    let mut heavy_query = mixed.query::<(&mut PositionComponent, &TransformComponent)>();
    let mut random_query = mixed.query::<&PositionComponent>();
    for &entity in &churn {
        mixed.entity_mut(entity).insert(Health(100.0));
    }
    assert!(churn
        .iter()
        .all(|&entity| mixed.get::<Health>(entity).is_some()));
    for &entity in &churn {
        mixed.entity_mut(entity).remove::<Health>();
    }

    let random_checksum = run_mixed_frame(
        &mut mixed,
        &mut move_query,
        &mut enemy_query,
        &mut ally_query,
        &mut heavy_query,
        &mut random_query,
        &random,
        &churn,
        &mut spawned,
    );
    assert_ne!(random_checksum, 0);
    assert_eq!(
        mixed.query::<&PositionComponent>().iter(&mixed).count(),
        expected
    );
    assert!(mixed.get::<Health>(churn[0]).is_none());
    assert!(spawned
        .iter()
        .all(|&entity| !mixed.entities().contains(entity)));

    let mut position_count = 0;
    let mut position_sum = 0.0;
    for position in mixed.query::<&PositionComponent>().iter(&mixed) {
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
    for health in mixed.query::<&Health>().iter(&mixed) {
        health_count += 1;
        health_sum += health.0;
    }
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);
}

use super::dense_iteration::world_with_entities;
use super::entity_insertion::{insert_native_bulk, native_bulk_context, prepared_insert_world};
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
    random_fragment_match_count, random_fragment_transition_shapes, DataComponent, Health,
    PositionComponent, RotationComponent, TagA, TagB, TagC, TagD, TagE, TagF, TagG, TagH,
    TransformComponent, VelocityComponent, A, B, C, CONTRACT_ENTITY_COUNT,
    CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT, D, E, ENTITY_OP_COUNT, F,
    FRAGMENTED_ENTITIES_PER_VARIANT, FRAGMENTED_VARIANT_COUNT, G, H, MIXED_FRAME_ALLIES,
    MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_MOVERS, MIXED_FRAME_SPAWN_COUNT,
    RANDOM_FRAGMENT_WORKLOADS,
};
use hecs::{PreparedQuery, World};

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
    let construction_inputs = distinct_suite_bundles(8);
    {
        let mut context = native_bulk_context(crate::common::suite_columns_from_bundles(
            &construction_inputs,
        ));
        assert_eq!(context.world.len(), 0);
        assert!(context.batch.is_some());
        insert_native_bulk(&mut context);
        assert!(context.batch.is_none());
        assert_eq!(context.world.len(), construction_inputs.len() as u32);
        let mut actual = context
            .world
            .query::<(
                &TransformComponent,
                &PositionComponent,
                &RotationComponent,
                &VelocityComponent,
            )>()
            .iter()
            .map(|(transform, position, rotation, velocity)| {
                (*transform, *position, *rotation, *velocity)
            })
            .collect::<Vec<_>>();
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
    {
        let mut construction_world = prepared_insert_world();
        assert_eq!(construction_world.len(), 0);
        for &bundle in &construction_inputs {
            construction_world.spawn(bundle);
        }
        let mut actual = construction_world
            .query::<(
                &TransformComponent,
                &PositionComponent,
                &RotationComponent,
                &VelocityComponent,
            )>()
            .iter()
            .map(|(transform, position, rotation, velocity)| {
                (*transform, *position, *rotation, *velocity)
            })
            .collect::<Vec<_>>();
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
}

fn validate_dense_iteration() {
    let mut batched_world = world_with_entities(CONTRACT_ENTITY_COUNT);
    super::dense_iteration::update_batched(&mut batched_world);
    assert_dense_update(&batched_world);

    let columns_world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let archetypes =
        super::dense_iteration::matching_archetypes(&columns_world, CONTRACT_ENTITY_COUNT);
    super::dense_iteration::update_archetype_columns(&archetypes);
    assert_dense_update(&columns_world);
}

fn assert_dense_update(world: &World) {
    assert_eq!(world.len(), CONTRACT_ENTITY_COUNT as u32);
    let positions = world
        .query::<&PositionComponent>()
        .iter()
        .map(|position| position.0.x)
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), CONTRACT_ENTITY_COUNT);
    assert_eq!(positions.iter().sum::<f32>(), 256.0);
}

fn validate_entity_lifecycle() {
    let mut world = world_with_entities(0);
    let entity = world.spawn(light_bundle());
    assert!(world.get::<&PositionComponent>(entity).is_ok());
    world.insert_one(entity, Health(100.0)).unwrap();
    assert!(world.get::<&Health>(entity).is_ok());
    world.remove_one::<Health>(entity).unwrap();
    assert!(world.get::<&Health>(entity).is_err());
    world.despawn(entity).unwrap();
    assert!(!world.contains(entity));
    assert!(world.get::<&PositionComponent>(entity).is_err());
}

fn validate_random_access() {
    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.spawn(light_bundle()))
        .collect();
    let random_orders = deterministic_orders(&random_entities);
    let mut random_query = PreparedQuery::<&PositionComponent>::default();
    let random_view = random_query.view_mut(&mut random_world);
    for order in random_orders {
        let random_checksum = order.iter().fold(0_u64, |checksum, &entity| {
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
    }
}

fn validate_fragmented_iteration() {
    let fragmented = fragmented_world();
    for data in fragmented.query::<&mut DataComponent>().iter() {
        data.0 = -data.0;
    }
    assert_eq!(
        fragmented
            .query::<&DataComponent>()
            .iter()
            .filter(|data| data.0 == -1.0)
            .count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
}

fn validate_random_fragmented_iteration() {
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        let (random_fragmented, masks) = random_fragmented_component_world(
            component_count,
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT,
        );
        assert_eq!(
            random_fragmented.archetypes().len(),
            random_fragment_transition_shapes(&masks, component_count).len()
        );
        let expected = random_fragment_match_count(&masks, term_count);
        let (matched, values, checksum) = match term_count {
            1 => {
                let mut result = (0, 0.0, 0_u64);
                for (entity, a) in random_fragmented.query::<(hecs::Entity, &A)>().iter() {
                    result.0 += 1;
                    result.1 += a.0;
                    result.2 = add_random_fragment_component_1_checksum(
                        result.2,
                        entity.to_bits().get(),
                        a.0,
                    );
                }
                result
            }
            4 => {
                let mut result = (0, 0.0, 0_u64);
                for (entity, a, b, c, d) in random_fragmented
                    .query::<(hecs::Entity, &A, &B, &C, &D)>()
                    .iter()
                {
                    result.0 += 1;
                    result.1 += a.0 + b.0 + c.0 + d.0;
                    result.2 = add_random_fragment_checksum(
                        result.2,
                        entity.to_bits().get(),
                        a.0,
                        b.0,
                        c.0,
                        d.0,
                    );
                }
                result
            }
            8 => {
                let mut result = (0, 0.0, 0_u64);
                for (entity, a, b, c, d, e, f, g, h) in random_fragmented
                    .query::<(hecs::Entity, &A, &B, &C, &D, &E, &F, &G, &H)>()
                    .iter()
                {
                    result.0 += 1;
                    result.1 += a.0 + b.0 + c.0 + d.0 + e.0 + f.0 + g.0 + h.0;
                    result.2 = add_random_fragment_component_8_checksum(
                        result.2,
                        entity.to_bits().get(),
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
        let mut entity_keys = random_fragmented
            .iter()
            .map(|entity| entity.entity().to_bits().get())
            .collect::<Vec<_>>();
        entity_keys.sort_unstable();
        assert_eq!(
            checksum,
            expected_random_fragment_checksum(&entity_keys, &masks, term_count)
        );

        let (random_tags, masks) =
            random_fragmented_tag_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        assert_eq!(
            random_tags.archetypes().len(),
            random_fragment_transition_shapes(&masks, component_count).len()
        );
        let tag_matches = match term_count {
            1 => random_tags.query::<&TagA>().iter().count(),
            4 => random_tags
                .query::<(&TagA, &TagB, &TagC, &TagD)>()
                .iter()
                .count(),
            8 => random_tags
                .query::<(&TagA, &TagB, &TagC, &TagD, &TagE, &TagF, &TagG, &TagH)>()
                .iter()
                .count(),
            _ => unreachable!(),
        };
        assert_eq!(tag_matches, random_fragment_match_count(&masks, term_count));
    }
}

fn validate_structural_changes() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let base_count = world.len();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    assert_eq!(world.len(), base_count + ENTITY_OP_COUNT as u32);
    let (add_order, remove_order) = component_change_orders(entity_ops.len());
    add_remove_health(&mut world, &entity_ops, &add_order, &remove_order);
    assert!(entity_ops
        .iter()
        .all(|&entity| world.get::<&Health>(entity).is_err()));
    let deletion_order = entity_deletion_order(entity_ops.len());
    despawn_entities(&mut world, &entity_ops, &deletion_order);
    assert_eq!(world.len(), base_count);
    assert!(entity_ops.iter().all(|&entity| !world.contains(entity)));
}

fn validate_mixed_frame() {
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

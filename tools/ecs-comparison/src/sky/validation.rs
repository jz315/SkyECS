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
    generational_entity_key, light_bundle, position_checksum_value, random_fragment_match_count,
    random_fragment_transition_shapes, DataComponent, Health, PositionComponent, RotationComponent,
    TagA, TagB, TagC, TagD, TagE, TagF, TagG, TagH, TransformComponent, VelocityComponent, A, B, C,
    CONTRACT_ENTITY_COUNT, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT, D, E, ENTITY_OP_COUNT, F,
    FRAGMENTED_ENTITIES_PER_VARIANT, FRAGMENTED_VARIANT_COUNT, G, H, MIXED_FRAME_ALLIES,
    MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_MOVERS, MIXED_FRAME_SPAWN_COUNT,
    RANDOM_FRAGMENT_WORKLOADS,
};
use sky_ecs::{PreparedQuery, World};

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
        assert_eq!(context.world.entity_count(), 0);
        insert_native_bulk(&mut context);
        assert!(context.columns.0.is_empty());
        assert!(context.columns.1.is_empty());
        assert!(context.columns.2.is_empty());
        assert!(context.columns.3.is_empty());
        assert_eq!(context.world.entity_count(), construction_inputs.len());
        let mut actual = Vec::new();
        PreparedQuery::<(
            &TransformComponent,
            &PositionComponent,
            &RotationComponent,
            &VelocityComponent,
        )>::new()
        .for_each(
            &mut context.world,
            |(transform, position, rotation, velocity)| {
                actual.push((*transform, *position, *rotation, *velocity));
            },
        );
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
    {
        let mut construction_world = prepared_insert_world();
        assert_eq!(construction_world.entity_count(), 0);
        for &bundle in &construction_inputs {
            construction_world.spawn(bundle);
        }
        assert_eq!(construction_world.entity_count(), construction_inputs.len());
        let mut actual = Vec::new();
        PreparedQuery::<(
            &TransformComponent,
            &PositionComponent,
            &RotationComponent,
            &VelocityComponent,
        )>::new()
        .for_each(
            &mut construction_world,
            |(transform, position, rotation, velocity)| {
                actual.push((*transform, *position, *rotation, *velocity));
            },
        );
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
}

fn validate_dense_iteration() {
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
}

fn validate_entity_lifecycle() {
    let mut world = world_with_entities(0);
    let entity = world.spawn(light_bundle());
    assert!(world.get::<PositionComponent>(entity).is_some());
    assert!(world.insert(entity, Health(100.0)));
    assert!(world.get::<Health>(entity).is_some());
    assert!(world.remove::<Health>(entity));
    assert!(world.get::<Health>(entity).is_none());
    assert!(world.despawn(entity));
    assert!(!world.contains(entity));
    assert!(world.get::<PositionComponent>(entity).is_none());
}

fn validate_random_access() {
    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.spawn(light_bundle()))
        .collect();
    let random_orders = deterministic_orders(&random_entities);
    let positions = random_world.accessor::<PositionComponent>();
    for order in random_orders {
        let random_checksum = order.iter().fold(0_u64, |checksum, &entity| {
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
    }
}

fn validate_fragmented_iteration() {
    let mut fragmented = fragmented_world();
    let mut fragmented_count = 0;
    PreparedQuery::<&mut DataComponent>::new().for_each(&mut fragmented, |data| {
        data.0 = -data.0;
        fragmented_count += 1;
    });
    assert_eq!(
        fragmented_count,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
    PreparedQuery::<&DataComponent>::new().for_each(&mut fragmented, |data| {
        assert_eq!(data.0, -1.0);
    });
}

fn validate_random_fragmented_iteration() {
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        let (mut random_fragmented, masks) = random_fragmented_component_world(
            component_count,
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT,
        );
        assert_eq!(
            random_fragmented.archetype_count(),
            random_fragment_transition_shapes(&masks, component_count).len()
        );
        let expected = random_fragment_match_count(&masks, term_count);
        let (matched, values, first_checksum, second_checksum) = match term_count {
            1 => {
                let mut query = PreparedQuery::<&A>::new();
                let mut run = || {
                    let mut matched = 0;
                    let mut values = 0.0;
                    let mut checksum = 0;
                    query.for_each_with_entity(&mut random_fragmented, |entity, a| {
                        matched += 1;
                        values += a.0;
                        checksum = add_random_fragment_component_1_checksum(
                            checksum,
                            generational_entity_key(entity.index(), entity.generation()),
                            a.0,
                        );
                    });
                    (matched, values, checksum)
                };
                let first = run();
                let second = run();
                (first.0, first.1, first.2, second.2)
            }
            4 => {
                let mut query = PreparedQuery::<(&A, &B, &C, &D)>::new();
                let mut run = || {
                    let mut matched = 0;
                    let mut values = 0.0;
                    let mut checksum = 0;
                    query.for_each_with_entity(&mut random_fragmented, |entity, (a, b, c, d)| {
                        matched += 1;
                        values += a.0 + b.0 + c.0 + d.0;
                        checksum = add_random_fragment_checksum(
                            checksum,
                            generational_entity_key(entity.index(), entity.generation()),
                            a.0,
                            b.0,
                            c.0,
                            d.0,
                        );
                    });
                    (matched, values, checksum)
                };
                let first = run();
                let second = run();
                (first.0, first.1, first.2, second.2)
            }
            8 => {
                let mut query = PreparedQuery::<(&A, &B, &C, &D, &E, &F, &G, &H)>::new();
                let mut run = || {
                    let mut matched = 0;
                    let mut values = 0.0;
                    let mut checksum = 0;
                    query.for_each_with_entity(
                        &mut random_fragmented,
                        |entity, (a, b, c, d, e, f, g, h)| {
                            matched += 1;
                            values += a.0 + b.0 + c.0 + d.0 + e.0 + f.0 + g.0 + h.0;
                            checksum = add_random_fragment_component_8_checksum(
                                checksum,
                                generational_entity_key(entity.index(), entity.generation()),
                                a.0,
                                b.0,
                                c.0,
                                d.0,
                                e.0,
                                f.0,
                                g.0,
                                h.0,
                            );
                        },
                    );
                    (matched, values, checksum)
                };
                let first = run();
                let second = run();
                (first.0, first.1, first.2, second.2)
            }
            _ => unreachable!(),
        };
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * term_count as f32 * 10.0);
        assert_eq!(first_checksum, second_checksum);

        let (random_tags, tag_masks) =
            random_fragmented_tag_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        assert_eq!(
            random_tags.archetype_count(),
            random_fragment_transition_shapes(&tag_masks, component_count).len()
        );
        let tag_matches = match term_count {
            1 => PreparedQuery::<&TagA>::new().count(&random_tags),
            4 => PreparedQuery::<(&TagA, &TagB, &TagC, &TagD)>::new().count(&random_tags),
            8 => PreparedQuery::<(&TagA, &TagB, &TagC, &TagD, &TagE, &TagF, &TagG, &TagH)>::new()
                .count(&random_tags),
            _ => unreachable!(),
        };
        assert_eq!(
            tag_matches,
            random_fragment_match_count(&tag_masks, term_count)
        );
    }
}

fn validate_structural_changes() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let base_count = world.entity_count();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    assert_eq!(world.entity_count(), base_count + ENTITY_OP_COUNT);
    let (add_order, remove_order) = component_change_orders(entity_ops.len());
    add_remove_health(&mut world, &entity_ops, &add_order, &remove_order);
    assert!(entity_ops
        .iter()
        .all(|&entity| world.get::<Health>(entity).is_none()));
    let deletion_order = entity_deletion_order(entity_ops.len());
    despawn_entities(&mut world, &entity_ops, &deletion_order);
    assert_eq!(world.entity_count(), base_count);
    assert!(entity_ops.iter().all(|&entity| !world.contains(entity)));
}

fn validate_mixed_frame() {
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

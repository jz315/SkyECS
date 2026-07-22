use super::dense_iteration::world_with_entities;
use super::entity_insertion::{insert_native_bulk, native_bulk_context, prepared_insert_world};
use super::fragmented_iteration::fragmented_world;
use super::mixed_frame::{
    mixed_churn_step, mixed_health_step, mixed_heavy_step, mixed_move_step, mixed_random_step,
    mixed_spawn_step, mixed_world,
};
use super::random_fragmented_iteration::{
    random_fragmented_component_world, random_fragmented_tag_world,
};
use super::structural_changes::{add_remove_health, despawn_entities};
use crate::common::{
    add_position_checksum, add_random_fragment_checksum, add_random_fragment_component_1_checksum,
    add_random_fragment_component_8_checksum, assert_approx_eq, assert_suite_bundles_match,
    component_change_orders, deterministic_orders, distinct_suite_bundles, entity_deletion_order,
    expected_random_fragment_checksum, light_bundle, position_checksum_value,
    random_fragment_match_count, DataComponent, Health, PositionComponent, RotationComponent, TagA,
    TagB, TagC, TagD, TagE, TagF, TagG, TagH, TagI, TagJ, TagK, TagL, TagM, TagN, TagO, TagP,
    TransformComponent, VelocityComponent, A, B, C, CONTRACT_ENTITY_COUNT,
    CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT, D, E, ENTITY_OP_COUNT, F,
    FRAGMENTED_ENTITIES_PER_VARIANT, FRAGMENTED_VARIANT_COUNT, G, H, I, J, K, L, M,
    MIXED_FRAME_ALLIES, MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY, MIXED_FRAME_MOVERS,
    MIXED_FRAME_SPAWN_COUNT, N, O, P, RANDOM_FRAGMENT_WORKLOADS,
};
use shipyard::{Get, IntoIter, View, ViewMut, World};

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
        assert_eq!(
            context
                .world
                .borrow::<shipyard::EntitiesView>()
                .unwrap()
                .iter()
                .count(),
            0
        );
        insert_native_bulk(&mut context);
        assert!(context.bundles.is_none());
        let (transforms, positions, rotations, velocities) = context
            .world
            .borrow::<(
                View<TransformComponent>,
                View<PositionComponent>,
                View<RotationComponent>,
                View<VelocityComponent>,
            )>()
            .unwrap();
        let mut actual = (&transforms, &positions, &rotations, &velocities)
            .iter()
            .map(|(transform, position, rotation, velocity)| {
                (*transform, *position, *rotation, *velocity)
            })
            .collect::<Vec<_>>();
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
    {
        let mut construction_world = prepared_insert_world();
        let construction_entities = construction_inputs
            .iter()
            .copied()
            .map(|bundle| construction_world.add_entity(bundle))
            .collect::<Vec<_>>();
        assert_eq!(construction_entities.len(), construction_inputs.len());
        let (transforms, positions, rotations, velocities) = construction_world
            .borrow::<(
                View<TransformComponent>,
                View<PositionComponent>,
                View<RotationComponent>,
                View<VelocityComponent>,
            )>()
            .unwrap();
        let mut actual = construction_entities
            .iter()
            .map(|&entity| {
                (
                    *(&transforms).get(entity).unwrap(),
                    *(&positions).get(entity).unwrap(),
                    *(&rotations).get(entity).unwrap(),
                    *(&velocities).get(entity).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_suite_bundles_match(&mut actual, &construction_inputs);
    }
}

fn validate_dense_iteration() {
    let world = world_with_entities(CONTRACT_ENTITY_COUNT);
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
}

fn validate_entity_lifecycle() {
    let mut world = World::new();
    let entity = world.add_entity(light_bundle());
    assert!(world.is_entity_alive(entity));
    world.add_component(entity, (Health(100.0),));
    assert!(world.borrow::<View<Health>>().unwrap().get(entity).is_ok());
    world.delete_component::<(Health,)>(entity);
    assert!(world.borrow::<View<Health>>().unwrap().get(entity).is_err());
    assert!(world.delete_entity(entity));
    assert!(!world.is_entity_alive(entity));
}

fn validate_random_access() {
    let mut random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| random_world.add_entity(light_bundle()))
        .collect();
    let random_orders = deterministic_orders(&random_entities);
    let positions = random_world.borrow::<View<PositionComponent>>().unwrap();
    for order in random_orders {
        let random_checksum = order.iter().fold(0_u64, |checksum, &entity| {
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

        let fixed_plan: Vec<_> = order
            .iter()
            .map(|&entity| {
                (&positions)
                    .get(entity)
                    .expect("contract entity must be readable through View")
            })
            .collect();
        let fixed_checksum = fixed_plan.into_iter().fold(0_u64, add_position_checksum);
        assert_eq!(
            fixed_checksum,
            position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
        );
    }
}

fn validate_fragmented_iteration() {
    let fragmented = fragmented_world();
    {
        let mut data = fragmented.borrow::<ViewMut<DataComponent>>().unwrap();
        (&mut data).iter().for_each(|value| value.0 = -value.0);
    }
    let data = fragmented.borrow::<View<DataComponent>>().unwrap();
    assert_eq!(
        (&data).iter().filter(|value| value.0 == -1.0).count(),
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
    drop(data);
}

fn validate_random_fragmented_iteration() {
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        let (random_fragmented, masks) = random_fragmented_component_world(
            component_count,
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT,
        );
        let entities = random_fragmented
            .borrow::<shipyard::EntitiesView>()
            .unwrap()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(entities.len(), masks.len());
        macro_rules! assert_component_membership {
            ($bit:expr, $component:ty) => {
                if $bit < component_count {
                    let view = random_fragmented.borrow::<View<$component>>().unwrap();
                    for (&entity, &mask) in entities.iter().zip(&masks) {
                        assert_eq!((&view).get(entity).is_ok(), mask & (1 << $bit) != 0);
                    }
                }
            };
        }
        assert_component_membership!(0, A);
        assert_component_membership!(1, B);
        assert_component_membership!(2, C);
        assert_component_membership!(3, D);
        assert_component_membership!(4, E);
        assert_component_membership!(5, F);
        assert_component_membership!(6, G);
        assert_component_membership!(7, H);
        assert_component_membership!(8, I);
        assert_component_membership!(9, J);
        assert_component_membership!(10, K);
        assert_component_membership!(11, L);
        assert_component_membership!(12, M);
        assert_component_membership!(13, N);
        assert_component_membership!(14, O);
        assert_component_membership!(15, P);
        let expected = random_fragment_match_count(&masks, term_count);
        let (matched, values, checksum) = match term_count {
            1 => {
                let a = random_fragmented.borrow::<View<A>>().unwrap();
                let mut result = (0, 0.0, 0_u64);
                a.iter().with_id().for_each(|(entity, a)| {
                    result.0 += 1;
                    result.1 += a.0;
                    result.2 =
                        add_random_fragment_component_1_checksum(result.2, entity.inner(), a.0);
                });
                result
            }
            4 => {
                let (a, b, c, d) = random_fragmented
                    .borrow::<(View<A>, View<B>, View<C>, View<D>)>()
                    .unwrap();
                let mut result = (0, 0.0, 0_u64);
                (&a, &b, &c, &d)
                    .iter()
                    .with_id()
                    .for_each(|(entity, (a, b, c, d))| {
                        result.0 += 1;
                        result.1 += a.0 + b.0 + c.0 + d.0;
                        result.2 = add_random_fragment_checksum(
                            result.2,
                            entity.inner(),
                            a.0,
                            b.0,
                            c.0,
                            d.0,
                        );
                    });
                result
            }
            8 => {
                let (a, b, c, d, e, f, g, h) = random_fragmented
                    .borrow::<(
                        View<A>,
                        View<B>,
                        View<C>,
                        View<D>,
                        View<E>,
                        View<F>,
                        View<G>,
                        View<H>,
                    )>()
                    .unwrap();
                let mut result = (0, 0.0, 0_u64);
                (&a, &b, &c, &d, &e, &f, &g, &h).iter().with_id().for_each(
                    |(entity, (a, b, c, d, e, f, g, h))| {
                        result.0 += 1;
                        result.1 += a.0 + b.0 + c.0 + d.0 + e.0 + f.0 + g.0 + h.0;
                        result.2 = add_random_fragment_component_8_checksum(
                            result.2,
                            entity.inner(),
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
                result
            }
            _ => unreachable!(),
        };
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * term_count as f32 * 10.0);
        let entity_keys = entities
            .iter()
            .map(|entity| entity.inner())
            .collect::<Vec<_>>();
        assert_eq!(
            checksum,
            expected_random_fragment_checksum(&entity_keys, &masks, term_count)
        );

        let (random_tags, masks) =
            random_fragmented_tag_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        let entities = random_tags
            .borrow::<shipyard::EntitiesView>()
            .unwrap()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(entities.len(), masks.len());
        macro_rules! assert_tag_membership {
            ($bit:expr, $component:ty) => {
                if $bit < component_count {
                    let view = random_tags.borrow::<View<$component>>().unwrap();
                    for (&entity, &mask) in entities.iter().zip(&masks) {
                        assert_eq!((&view).get(entity).is_ok(), mask & (1 << $bit) != 0);
                    }
                }
            };
        }
        assert_tag_membership!(0, TagA);
        assert_tag_membership!(1, TagB);
        assert_tag_membership!(2, TagC);
        assert_tag_membership!(3, TagD);
        assert_tag_membership!(4, TagE);
        assert_tag_membership!(5, TagF);
        assert_tag_membership!(6, TagG);
        assert_tag_membership!(7, TagH);
        assert_tag_membership!(8, TagI);
        assert_tag_membership!(9, TagJ);
        assert_tag_membership!(10, TagK);
        assert_tag_membership!(11, TagL);
        assert_tag_membership!(12, TagM);
        assert_tag_membership!(13, TagN);
        assert_tag_membership!(14, TagO);
        assert_tag_membership!(15, TagP);
        let tag_matches = match term_count {
            1 => random_tags.borrow::<View<TagA>>().unwrap().iter().count(),
            4 => {
                let (a, b, c, d) = random_tags
                    .borrow::<(View<TagA>, View<TagB>, View<TagC>, View<TagD>)>()
                    .unwrap();
                (&a, &b, &c, &d).iter().count()
            }
            8 => {
                let (a, b, c, d, e, f, g, h) = random_tags
                    .borrow::<(
                        View<TagA>,
                        View<TagB>,
                        View<TagC>,
                        View<TagD>,
                        View<TagE>,
                        View<TagF>,
                        View<TagG>,
                        View<TagH>,
                    )>()
                    .unwrap();
                (&a, &b, &c, &d, &e, &f, &g, &h).iter().count()
            }
            _ => unreachable!(),
        };
        assert_eq!(tag_matches, random_fragment_match_count(&masks, term_count));
    }
}

fn validate_structural_changes() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
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
    let (add_order, remove_order) = component_change_orders(entity_ops.len());
    add_remove_health(&mut world, &entity_ops, &add_order, &remove_order);
    {
        let health = world.borrow::<View<Health>>().unwrap();
        assert!(entity_ops
            .iter()
            .all(|&entity| (&health).get(entity).is_err()));
    }
    let deletion_order = entity_deletion_order(entity_ops.len());
    despawn_entities(&mut world, &entity_ops, &deletion_order);
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
}

fn validate_mixed_frame() {
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

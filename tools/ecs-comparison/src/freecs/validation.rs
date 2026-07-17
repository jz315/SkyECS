use super::dense_iteration::world_with_entities;
use super::entity_insertion::{
    prepared_insert_world, spawn_suite_bundle, spawn_suite_bundles, World, A_MASK, B_MASK, C_MASK,
    DATA_MASK, D_MASK, E_MASK, F_MASK, G_MASK, HEALTH_MASK, H_MASK, MOVE_MASK, POSITION_MASK,
    SUITE_MASK, TAG_A_MASK, TAG_B_MASK, TAG_C_MASK, TAG_D_MASK, TAG_E_MASK, TAG_F_MASK, TAG_G_MASK,
    TAG_H_MASK,
};
use super::fragmented_iteration::fragmented_world;
use super::mixed_frame::{mixed_world, run_mixed_frame};
use super::random_fragmented_iteration::{
    random_fragmented_component_world, random_fragmented_tag_world,
};
use super::structural_changes::{
    add_remove_health, despawn_entities, spawn_light_batch, spawn_light_one,
};
use crate::common::{
    add_position_checksum, add_random_fragment_checksum, add_random_fragment_component_1_checksum,
    add_random_fragment_component_8_checksum, assert_approx_eq, assert_suite_bundles_match,
    component_change_orders, deterministic_orders, distinct_suite_bundles, entity_deletion_order,
    expected_random_fragment_checksum, generational_entity_key, position_checksum_value,
    random_fragment_match_count, Health, CONTRACT_ENTITY_COUNT,
    CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT, ENTITY_OP_COUNT, FRAGMENTED_ENTITIES_PER_VARIANT,
    FRAGMENTED_VARIANT_COUNT, MIXED_FRAME_ALLIES, MIXED_FRAME_ENEMIES, MIXED_FRAME_HEAVY,
    MIXED_FRAME_MOVERS, MIXED_FRAME_SPAWN_COUNT, RANDOM_FRAGMENT_WORKLOADS,
};
use freecs::Entity;

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
    let mut construction_world = prepared_insert_world();
    assert_eq!(construction_world.entity_count(), 0);
    let construction_bundles = distinct_suite_bundles(8);
    let mut construction_entities =
        spawn_suite_bundles(&mut construction_world, &construction_bundles);
    let single_inputs = distinct_suite_bundles(3);
    construction_entities.extend(
        single_inputs
            .iter()
            .copied()
            .map(|bundle| spawn_suite_bundle(&mut construction_world, bundle)),
    );
    assert_eq!(
        construction_world.entity_count(),
        construction_bundles.len() + single_inputs.len()
    );
    let mut construction_count = 0;
    let mut actual = Vec::new();
    construction_world.for_each(SUITE_MASK, 0, |_entity, table, index| {
        construction_count += 1;
        actual.push((
            table.transform[index],
            table.position[index],
            table.rotation[index],
            table.velocity[index],
        ));
    });
    assert_eq!(construction_count, construction_entities.len());
    let mut expected = construction_bundles;
    expected.extend(single_inputs);
    assert_suite_bundles_match(&mut actual, &expected);
}

fn validate_dense_iteration() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    assert_eq!(world.entity_count(), CONTRACT_ENTITY_COUNT);
    let mut count = 0;
    let mut checksum = 0.0;
    world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
        table.position[index].0 += table.velocity[index].0;
        count += 1;
        checksum += table.position[index].0.x;
    });
    assert_eq!(count, CONTRACT_ENTITY_COUNT);
    assert_eq!(checksum, 256.0);
}

fn validate_entity_lifecycle() {
    let mut world = world_with_entities(0);
    let entity = spawn_light_one(&mut world);
    assert!(world.contains_entity(entity));
    assert!(world.get_position(entity).is_some());
    world.set_health(entity, Health(100.0));
    assert!(world.get_health(entity).is_some());
    assert!(world.remove_health(entity));
    assert!(world.get_health(entity).is_none());
    assert_eq!(world.despawn_entities(&[entity]), vec![entity]);
    assert!(!world.contains_entity(entity));
    assert!(world.get_position(entity).is_none());
}

fn validate_random_access() {
    let mut random_world = World::default();
    let random_entities = spawn_light_batch(&mut random_world, CONTRACT_ENTITY_COUNT);
    for order in deterministic_orders(&random_entities) {
        let random_checksum = order.iter().fold(0_u64, |checksum, &entity| {
            add_position_checksum(
                checksum,
                random_world
                    .get_position(entity)
                    .expect("contract entity must be readable through generated getter"),
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
    fragmented.for_each_data_mut(|data| data.0 = -data.0);
    let mut fragmented_count = 0;
    fragmented.for_each(DATA_MASK, 0, |_, table, index| {
        assert_eq!(table.data[index].0, -1.0);
        fragmented_count += 1;
    });
    assert_eq!(
        fragmented_count,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
}

fn validate_random_fragmented_iteration() {
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        let validation_entity_count = if component_count == 16 {
            CONTRACT_ENTITY_COUNT
        } else {
            CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT
        };
        let (random_fragmented, masks) =
            random_fragmented_component_world(component_count, validation_entity_count);
        for (id, &mask) in masks.iter().enumerate() {
            let entity = Entity {
                id: id as u32,
                generation: 0,
            };
            macro_rules! assert_component {
                ($bit:expr, $getter:ident) => {
                    assert_eq!(
                        random_fragmented.$getter(entity).is_some(),
                        mask & (1 << $bit) != 0
                    );
                };
            }
            assert_component!(0, get_fragment_a);
            assert_component!(1, get_fragment_b);
            assert_component!(2, get_fragment_c);
            assert_component!(3, get_fragment_d);
            assert_component!(4, get_fragment_e);
            assert_component!(5, get_fragment_f);
            assert_component!(6, get_fragment_g);
            assert_component!(7, get_fragment_h);
            assert_component!(8, get_fragment_i);
            assert_component!(9, get_fragment_j);
            assert_component!(10, get_fragment_k);
            assert_component!(11, get_fragment_l);
            assert_component!(12, get_fragment_m);
            assert_component!(13, get_fragment_n);
            assert_component!(14, get_fragment_o);
            assert_component!(15, get_fragment_p);
        }
        let expected = random_fragment_match_count(&masks, term_count);
        let query_mask = match term_count {
            1 => A_MASK,
            4 => A_MASK | B_MASK | C_MASK | D_MASK,
            8 => A_MASK | B_MASK | C_MASK | D_MASK | E_MASK | F_MASK | G_MASK | H_MASK,
            _ => unreachable!(),
        };
        let mut matched = 0;
        let mut values = 0.0;
        let mut checksum = 0_u64;
        random_fragmented.for_each(query_mask, 0, |entity, table, index| {
            matched += 1;
            match term_count {
                1 => {
                    values += table.fragment_a[index].0;
                    checksum = add_random_fragment_component_1_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                    );
                }
                4 => {
                    values += table.fragment_a[index].0
                        + table.fragment_b[index].0
                        + table.fragment_c[index].0
                        + table.fragment_d[index].0;
                    checksum = add_random_fragment_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                        table.fragment_b[index].0,
                        table.fragment_c[index].0,
                        table.fragment_d[index].0,
                    );
                }
                8 => {
                    values += table.fragment_a[index].0
                        + table.fragment_b[index].0
                        + table.fragment_c[index].0
                        + table.fragment_d[index].0
                        + table.fragment_e[index].0
                        + table.fragment_f[index].0
                        + table.fragment_g[index].0
                        + table.fragment_h[index].0;
                    checksum = add_random_fragment_component_8_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                        table.fragment_b[index].0,
                        table.fragment_c[index].0,
                        table.fragment_d[index].0,
                        table.fragment_e[index].0,
                        table.fragment_f[index].0,
                        table.fragment_g[index].0,
                        table.fragment_h[index].0,
                    );
                }
                _ => unreachable!(),
            }
        });
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * term_count as f32 * 10.0);
        let entity_keys = (0..masks.len())
            .map(|id| generational_entity_key(id as u32, 0))
            .collect::<Vec<_>>();
        assert_eq!(
            checksum,
            expected_random_fragment_checksum(&entity_keys, &masks, term_count)
        );

        let (random_tags, masks) =
            random_fragmented_tag_world(component_count, validation_entity_count);
        for (id, &mask) in masks.iter().enumerate() {
            let entity = Entity {
                id: id as u32,
                generation: 0,
            };
            macro_rules! assert_tag {
                ($bit:expr, $getter:ident) => {
                    assert_eq!(
                        random_tags.$getter(entity).is_some(),
                        mask & (1 << $bit) != 0
                    );
                };
            }
            assert_tag!(0, get_tag_a);
            assert_tag!(1, get_tag_b);
            assert_tag!(2, get_tag_c);
            assert_tag!(3, get_tag_d);
            assert_tag!(4, get_tag_e);
            assert_tag!(5, get_tag_f);
            assert_tag!(6, get_tag_g);
            assert_tag!(7, get_tag_h);
            assert_tag!(8, get_tag_i);
            assert_tag!(9, get_tag_j);
            assert_tag!(10, get_tag_k);
            assert_tag!(11, get_tag_l);
            assert_tag!(12, get_tag_m);
            assert_tag!(13, get_tag_n);
            assert_tag!(14, get_tag_o);
            assert_tag!(15, get_tag_p);
        }
        let tag_mask = match term_count {
            1 => TAG_A_MASK,
            4 => TAG_A_MASK | TAG_B_MASK | TAG_C_MASK | TAG_D_MASK,
            8 => {
                TAG_A_MASK
                    | TAG_B_MASK
                    | TAG_C_MASK
                    | TAG_D_MASK
                    | TAG_E_MASK
                    | TAG_F_MASK
                    | TAG_G_MASK
                    | TAG_H_MASK
            }
            _ => unreachable!(),
        };
        let mut tag_matches = 0;
        random_tags.for_each(tag_mask, 0, |_, _, _| tag_matches += 1);
        assert_eq!(tag_matches, random_fragment_match_count(&masks, term_count));
    }
}

fn validate_structural_changes() {
    let mut world = world_with_entities(CONTRACT_ENTITY_COUNT);
    let base_count = world.entity_count();
    let entity_ops: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| spawn_light_one(&mut world))
        .collect();
    assert_eq!(world.entity_count(), base_count + ENTITY_OP_COUNT);
    let (add_order, remove_order) = component_change_orders(entity_ops.len());
    add_remove_health(&mut world, &entity_ops, &add_order, &remove_order);
    assert!(entity_ops
        .iter()
        .all(|&entity| world.get_health(entity).is_none()));
    let deletion_order = entity_deletion_order(entity_ops.len());
    despawn_entities(&mut world, &entity_ops, &deletion_order);
    assert_eq!(world.entity_count(), base_count);
    assert!(entity_ops
        .iter()
        .all(|&entity| !world.contains_entity(entity)));
}

fn validate_mixed_frame() {
    let (mut mixed, random, churn) = mixed_world();
    let expected = mixed.entity_count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    for &entity in &churn {
        mixed.set_health(entity, Health(100.0));
    }
    assert!(churn
        .iter()
        .all(|&entity| mixed.get_health(entity).is_some()));
    for &entity in &churn {
        assert!(mixed.remove_health(entity));
    }

    let random_checksum = run_mixed_frame(&mut mixed, &random, &churn, &mut spawned);
    assert_ne!(random_checksum, 0);
    assert_eq!(mixed.entity_count(), expected);
    assert!(mixed.get_health(churn[0]).is_none());
    assert!(spawned.iter().all(|&entity| !mixed.contains_entity(entity)));

    let mut position_count = 0;
    let mut position_sum = 0.0;
    mixed.for_each(POSITION_MASK, 0, |_, table, index| {
        position_count += 1;
        position_sum += table.position[index].0.x;
    });
    assert_eq!(
        position_count,
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
    );
    assert_approx_eq(position_sum, 18_500.0);

    let mut health_count = 0;
    let mut health_sum = 0.0;
    mixed.for_each(HEALTH_MASK, 0, |_, table, index| {
        health_count += 1;
        health_sum += table.health[index].0;
    });
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);
}

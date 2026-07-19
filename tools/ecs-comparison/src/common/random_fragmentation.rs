use super::{PositionComponent, RANDOM_FRAGMENT_ENTITY_COUNT, RANDOM_ORDER_COUNT};
use std::collections::BTreeSet;

pub fn deterministic_shuffle<T>(slice: &mut [T]) {
    deterministic_shuffle_with_seed(slice, 0xDEAD_BEEF_CAFE_BABE);
}

pub fn deterministic_shuffle_with_seed<T>(slice: &mut [T], mut state: u64) {
    for i in (1..slice.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        slice.swap(i, j);
    }
}

pub fn deterministic_orders<T: Copy>(entities: &[T]) -> Vec<Vec<T>> {
    (0..RANDOM_ORDER_COUNT)
        .map(|order| {
            let mut shuffled = entities.to_vec();
            deterministic_shuffle_with_seed(
                &mut shuffled,
                0xDEAD_BEEF_CAFE_BABE ^ (order as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            shuffled
        })
        .collect()
}

const ENTITY_DELETION_SEED: u64 = 0xDEAD_BEEF_CAFE_BABE;
const COMPONENT_ADD_SEED: u64 = 0xA076_1D64_78BD_642F;
const COMPONENT_REMOVE_SEED: u64 = 0xE703_7ED1_A0B4_28DB;

fn entity_order_with_seed(entity_count: usize, seed: u64) -> Vec<usize> {
    let mut order = (0..entity_count).collect::<Vec<_>>();
    deterministic_shuffle_with_seed(&mut order, seed);
    order
}

pub fn entity_deletion_order(entity_count: usize) -> Vec<usize> {
    entity_order_with_seed(entity_count, ENTITY_DELETION_SEED)
}

pub fn component_change_orders(entity_count: usize) -> (Vec<usize>, Vec<usize>) {
    (
        entity_order_with_seed(entity_count, COMPONENT_ADD_SEED),
        entity_order_with_seed(entity_count, COMPONENT_REMOVE_SEED),
    )
}

pub fn sample_entities<T: Copy>(entities: &[T], count: usize) -> Vec<T> {
    assert!(count > 0);
    assert!(entities.len() >= count);
    let mut sampled = (0..count)
        .map(|index| entities[index * entities.len() / count])
        .collect::<Vec<_>>();
    deterministic_shuffle(&mut sampled);
    sampled
}

#[inline(always)]
pub fn add_position_checksum(checksum: u64, position: &PositionComponent) -> u64 {
    checksum.wrapping_add(position.0.x.to_bits() as u64)
}

#[inline(always)]
pub fn add_full_position_checksum(checksum: u64, position: &PositionComponent) -> u64 {
    checksum
        .wrapping_add(position.0.x.to_bits() as u64)
        .wrapping_add(position.0.y.to_bits() as u64)
        .wrapping_add(position.0.z.to_bits() as u64)
}

pub fn position_checksum_value(position_x: f32, count: usize) -> u64 {
    (position_x.to_bits() as u64).wrapping_mul(count as u64)
}

#[inline(always)]
pub fn generational_entity_key(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

#[inline(always)]
pub fn add_random_fragment_checksum(
    checksum: u64,
    entity_key: u64,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
) -> u64 {
    checksum
        .wrapping_add(entity_key)
        .wrapping_add(a as u64)
        .wrapping_add(b as u64)
        .wrapping_add(c as u64)
        .wrapping_add(d as u64)
}

#[inline(always)]
pub fn add_random_fragment_component_1_checksum(checksum: u64, entity_key: u64, a: f32) -> u64 {
    checksum.wrapping_add(entity_key).wrapping_add(a as u64)
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn add_random_fragment_component_8_checksum(
    checksum: u64,
    entity_key: u64,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    g: f32,
    h: f32,
) -> u64 {
    checksum
        .wrapping_add(entity_key)
        .wrapping_add(a as u64)
        .wrapping_add(b as u64)
        .wrapping_add(c as u64)
        .wrapping_add(d as u64)
        .wrapping_add(e as u64)
        .wrapping_add(f as u64)
        .wrapping_add(g as u64)
        .wrapping_add(h as u64)
}

pub fn assert_approx_eq(actual: f32, expected: f32) {
    let tolerance = expected.abs().max(1.0) * 1.0e-4;
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

pub fn random_fragment_masks(component_count: usize) -> Vec<u16> {
    random_fragment_masks_for(component_count, RANDOM_FRAGMENT_ENTITY_COUNT)
}

pub fn random_fragment_masks_for(component_count: usize, entity_count: usize) -> Vec<u16> {
    assert!((1..=16).contains(&component_count));
    let active_mask = if component_count == 16 {
        u16::MAX
    } else {
        (1_u16 << component_count) - 1
    };
    let mut state = 0x243F_6A88_85A3_08D3_u64;
    (0..entity_count)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = state;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            (value ^ (value >> 31)) as u16 & active_mask
        })
        .collect()
}

pub fn random_fragment_match_count(masks: &[u16], term_count: usize) -> usize {
    assert!(matches!(term_count, 1 | 4 | 8));
    let query_mask = (1_u16 << term_count) - 1;
    masks
        .iter()
        .filter(|&&mask| mask & query_mask == query_mask)
        .count()
}

pub fn expected_random_fragment_checksum(
    entity_keys: &[u64],
    masks: &[u16],
    term_count: usize,
) -> u64 {
    assert_eq!(entity_keys.len(), masks.len());
    let query_mask = (1_u16 << term_count) - 1;
    entity_keys
        .iter()
        .zip(masks)
        .filter(|(_, mask)| **mask & query_mask == query_mask)
        .fold(0_u64, |checksum, (&entity_key, _)| match term_count {
            1 => add_random_fragment_component_1_checksum(checksum, entity_key, 10.0),
            4 => add_random_fragment_checksum(checksum, entity_key, 10.0, 10.0, 10.0, 10.0),
            8 => add_random_fragment_component_8_checksum(
                checksum, entity_key, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,
            ),
            _ => unreachable!(),
        })
}

pub fn random_fragment_transition_shapes(masks: &[u16], component_count: usize) -> BTreeSet<u16> {
    assert!((1..=16).contains(&component_count));
    let mut shapes = BTreeSet::from([0]);
    for &mask in masks {
        let mut shape = 0_u16;
        for bit in 0..component_count {
            let component = 1_u16 << bit;
            if mask & component != 0 {
                shape |= component;
                shapes.insert(shape);
            }
        }
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::RANDOM_FRAGMENT_WORKLOADS;

    #[test]
    fn random_fragment_workload_is_stable() {
        let masks = random_fragment_masks(16);
        assert_eq!(masks.len(), RANDOM_FRAGMENT_ENTITY_COUNT);
        assert_eq!(random_fragment_match_count(&masks, 4), 4_103);
        for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
            let inactive_mask = if component_count == 16 {
                0
            } else {
                u16::MAX << component_count
            };
            let masks = random_fragment_masks(component_count);
            assert!(masks.iter().all(|mask| *mask & inactive_mask == 0));
            assert!(random_fragment_match_count(&masks, term_count) > 0);
        }
    }

    #[test]
    fn transition_shapes_include_ordered_prefixes() {
        let shapes = random_fragment_transition_shapes(&[0b1010, 0b0111], 4);
        assert_eq!(
            shapes,
            BTreeSet::from([0b0000, 0b0010, 0b0001, 0b0011, 0b0111, 0b1010])
        );
    }

    #[test]
    fn entity_deletion_order_is_a_stable_permutation() {
        let order = entity_deletion_order(1_000);
        assert_ne!(order, (0..1_000).collect::<Vec<_>>());

        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..1_000).collect::<Vec<_>>());
    }

    #[test]
    fn component_change_orders_are_distinct_permutations() {
        let (add_order, remove_order) = component_change_orders(1_000);
        assert_ne!(add_order, remove_order);

        for mut order in [add_order, remove_order] {
            order.sort_unstable();
            assert_eq!(order, (0..1_000).collect::<Vec<_>>());
        }
    }
}

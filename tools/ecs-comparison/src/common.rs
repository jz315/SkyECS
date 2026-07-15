// Shared components, constants, and helpers for all bench files.
// Include via: #[path = "../common.rs"] mod common; use common::*;

#![allow(dead_code)]

use cgmath::{Matrix4, Rad, Vector3};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const SIMPLE_ENTITY_COUNT: usize = 10_000;
pub const WARM_RANDOM_ENTITY_COUNT: usize = 100_000;
pub const COLD_RANDOM_ENTITY_COUNT: usize = 1_000_000;
pub const RANDOM_ORDER_COUNT: usize = 4;
pub const REPEATED_ITERATION_COUNT: usize = 32;
pub const LARGE_ITERATION_ENTITY_COUNT: usize = 100_000;
pub const FRAGMENTED_VARIANT_COUNT: usize = 26;
pub const FRAGMENTED_ENTITIES_PER_VARIANT: usize = 400;
pub const RANDOM_FRAGMENT_ENTITY_COUNT: usize = 65_536;
pub const RANDOM_FRAGMENT_COMPONENT_COUNTS: [usize; 4] = [6, 8, 10, 16];
pub const RANDOM_FRAGMENT_QUERY_MASK: u16 = 0b1111;
pub const HEAVY_ENTITY_COUNT: usize = 1_000;
pub const HEAVY_INVERT_COUNT: usize = 100;
pub const ENTITY_OP_COUNT: usize = 1_000;
pub const MIXED_FRAME_MOVERS: usize = 16_000;
pub const MIXED_FRAME_ENEMIES: usize = 4_000;
pub const MIXED_FRAME_ALLIES: usize = 4_000;
pub const MIXED_FRAME_HEAVY: usize = 1_000;
pub const MIXED_FRAME_RANDOM_COUNT: usize = 512;
pub const MIXED_FRAME_CHURN_COUNT: usize = 256;
pub const MIXED_FRAME_SPAWN_COUNT: usize = 64;
pub const MIXED_FRAME_INVERT_COUNT: usize = 8;
pub const MIXED_PHASE_HEALTH_REPEAT: usize = 8;
pub const MIXED_PHASE_SPAWN_REPEAT: usize = 32;
pub const CONTRACT_ENTITY_COUNT: usize = 128;
pub const CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT: usize = 2_048;

/// Entity count for system schedule benchmarks.
pub const SCHEDULE_ENTITY_COUNT: usize = 10_000;
/// System counts for scaling tests.
pub const SCHEDULE_SYSTEM_COUNTS: [usize; 3] = [1, 4, 16];

/// Entity count for the head-to-head hot-path benchmarks (sky vs hecs).
pub const HOT_PATH_ENTITY_COUNT: usize = 5_000_000;
pub const HOT_PATH_DELTA: f32 = 0.1;

// ---------------------------------------------------------------------------
// Components — used across all engines
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct TransformComponent(pub Matrix4<f32>);

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct PositionComponent(pub Vector3<f32>);

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct RotationComponent(pub Vector3<f32>);

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct VelocityComponent(pub Vector3<f32>);

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct DataComponent(pub f32);

impl Default for TransformComponent {
    fn default() -> Self {
        Self(Matrix4::from_scale(1.0))
    }
}

impl Default for PositionComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for RotationComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for VelocityComponent {
    fn default() -> Self {
        Self(Vector3::new(0.0, 0.0, 0.0))
    }
}

impl Default for DataComponent {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(
    Clone,
    Copy,
    Default,
    bevy_ecs::prelude::Component,
    flecs_ecs::prelude::Component,
    shipyard::Component,
)]
pub struct Health(pub f32);

#[derive(
    Clone,
    Copy,
    Default,
    bevy_ecs::prelude::Component,
    flecs_ecs::prelude::Component,
    shipyard::Component,
)]
pub struct Damage(pub f32);

#[derive(
    Clone,
    Copy,
    Default,
    bevy_ecs::prelude::Component,
    flecs_ecs::prelude::Component,
    shipyard::Component,
)]
pub struct Regen(pub f32);

#[derive(
    Clone,
    Copy,
    Default,
    bevy_ecs::prelude::Component,
    flecs_ecs::prelude::Component,
    shipyard::Component,
)]
pub struct IsEnemy;

#[derive(
    Clone,
    Copy,
    Default,
    bevy_ecs::prelude::Component,
    flecs_ecs::prelude::Component,
    shipyard::Component,
)]
pub struct IsAlly;

/// Lightweight 2-field components for the hot-path head-to-head benchmarks.
#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct Position2D {
    pub x: f32,
    pub y: f32,
}

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct Velocity2D {
    pub x: f32,
    pub y: f32,
}

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct AuxA {
    pub x: f32,
    pub y: f32,
}

#[derive(
    Clone, Copy, bevy_ecs::prelude::Component, flecs_ecs::prelude::Component, shipyard::Component,
)]
pub struct AuxB {
    pub x: f32,
    pub y: f32,
}

macro_rules! define_fragment_tags {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(
                Clone,
                Copy,
                Default,
                bevy_ecs::prelude::Component,
                flecs_ecs::prelude::Component,
                shipyard::Component,
            )]
            pub struct $name(pub f32);
        )+
    };
}

define_fragment_tags!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn suite_transform() -> TransformComponent {
    TransformComponent(Matrix4::from_scale(1.0))
}

pub fn suite_position() -> PositionComponent {
    PositionComponent(Vector3::unit_x())
}

pub fn suite_rotation() -> RotationComponent {
    RotationComponent(Vector3::unit_x())
}

pub fn suite_velocity() -> VelocityComponent {
    VelocityComponent(Vector3::unit_x())
}

pub fn heavy_matrix() -> Matrix4<f32> {
    Matrix4::<f32>::from_angle_x(Rad(1.2))
}

pub type SuiteBundle = (
    TransformComponent,
    PositionComponent,
    RotationComponent,
    VelocityComponent,
);

pub fn suite_bundle() -> SuiteBundle {
    (
        suite_transform(),
        suite_position(),
        suite_rotation(),
        suite_velocity(),
    )
}

pub fn suite_bundles(count: usize) -> Vec<SuiteBundle> {
    vec![suite_bundle(); count]
}

pub fn light_bundle() -> (PositionComponent, VelocityComponent) {
    (suite_position(), suite_velocity())
}

pub fn heavy_bundle() -> (
    TransformComponent,
    PositionComponent,
    RotationComponent,
    VelocityComponent,
) {
    (
        TransformComponent(heavy_matrix()),
        suite_position(),
        suite_rotation(),
        suite_velocity(),
    )
}

pub fn hot_path_bundle() -> (Velocity2D, Position2D, AuxA, AuxB) {
    (
        Velocity2D { x: 1.0, y: 1.0 },
        Position2D { x: 0.0, y: 0.0 },
        AuxA { x: 0.0, y: 0.0 },
        AuxB { x: 1.0, y: 1.0 },
    )
}

pub fn mixed_mover_bundle() -> (PositionComponent, VelocityComponent) {
    (
        PositionComponent(Vector3::new(0.0, 1.0, 0.0)),
        VelocityComponent(Vector3::new(1.0, 0.5, 0.25)),
    )
}

pub fn mixed_enemy_bundle() -> (
    PositionComponent,
    VelocityComponent,
    Health,
    Damage,
    IsEnemy,
) {
    (
        PositionComponent(Vector3::new(2.0, 0.0, 0.0)),
        VelocityComponent(Vector3::new(0.25, 1.0, 0.0)),
        Health(100.0),
        Damage(0.75),
        IsEnemy,
    )
}

pub fn mixed_ally_bundle() -> (PositionComponent, VelocityComponent, Health, Regen, IsAlly) {
    (
        PositionComponent(Vector3::new(-2.0, 0.0, 0.0)),
        VelocityComponent(Vector3::new(0.0, 0.75, 0.25)),
        Health(60.0),
        Regen(0.35),
        IsAlly,
    )
}

pub fn mixed_heavy_bundle() -> (TransformComponent, PositionComponent, VelocityComponent) {
    (
        TransformComponent(heavy_matrix()),
        PositionComponent(Vector3::new(1.0, 0.0, 0.0)),
        VelocityComponent(Vector3::new(0.5, 0.0, 0.5)),
    )
}

/// Deterministic Fisher-Yates shuffle using xorshift64.
/// Fixed seed so benchmark runs are reproducible without external deps.
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

#[inline(always)]
pub fn add_position_checksum(checksum: u64, position: &PositionComponent) -> u64 {
    checksum.wrapping_add(position.0.x.to_bits() as u64)
}

pub fn position_checksum_value(position_x: f32, count: usize) -> u64 {
    (position_x.to_bits() as u64).wrapping_mul(count as u64)
}

pub fn assert_approx_eq(actual: f32, expected: f32) {
    let tolerance = expected.abs().max(1.0) * 1.0e-4;
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

/// Reproducible counterpart of Sander Mertens' random-component fragmentation
/// workload: 65,536 entities independently receive subsets of 6, 8, 10, or 16 components.
///
/// Source: https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1719
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

pub fn random_fragment_match_count(masks: &[u16]) -> usize {
    masks
        .iter()
        .filter(|&&mask| mask & RANDOM_FRAGMENT_QUERY_MASK == RANDOM_FRAGMENT_QUERY_MASK)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_fragment_workload_is_stable() {
        let masks = random_fragment_masks(16);
        assert_eq!(masks.len(), RANDOM_FRAGMENT_ENTITY_COUNT);
        assert_eq!(random_fragment_match_count(&masks), 4_103);
        for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
            let inactive_mask = if component_count == 16 {
                0
            } else {
                u16::MAX << component_count
            };
            assert!(random_fragment_masks(component_count)
                .iter()
                .all(|mask| *mask & inactive_mask == 0));
        }
    }
}

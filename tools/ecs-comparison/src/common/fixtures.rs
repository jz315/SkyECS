use super::{
    AuxA, AuxB, BenchmarkMatrix, Damage, Health, IsAlly, IsEnemy, Position2D, PositionComponent,
    Regen, RotationComponent, TransformComponent, Velocity2D, VelocityComponent,
};
use cgmath::Vector3;

pub const SIMPLE_ENTITY_COUNT: usize = 10_000;
pub const WARM_RANDOM_ENTITY_COUNT: usize = 100_000;
pub const RANDOM_ORDER_COUNT: usize = 4;
pub const LARGE_ITERATION_ENTITY_COUNT: usize = 100_000;
pub const VERY_LARGE_ITERATION_ENTITY_COUNT: usize = 1_000_000;
pub const FRAGMENTED_VARIANT_COUNT: usize = 26;
pub const FRAGMENTED_ENTITIES_PER_VARIANT: usize = 400;
pub const RANDOM_FRAGMENT_ENTITY_COUNT: usize = 65_536;
pub const RANDOM_FRAGMENT_WORKLOADS: [(usize, usize); 10] = [
    (6, 1),
    (6, 4),
    (8, 1),
    (8, 4),
    (10, 1),
    (10, 4),
    (10, 8),
    (16, 1),
    (16, 4),
    (16, 8),
];
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
pub const SCHEDULE_ENTITY_COUNT: usize = 10_000;
pub const SCHEDULE_SYSTEM_COUNTS: [usize; 3] = [1, 4, 16];
pub const HOT_PATH_ENTITY_COUNT: usize = 5_000_000;
pub const HOT_PATH_DELTA: f32 = 0.1;

pub type SuiteBundle = (
    TransformComponent,
    PositionComponent,
    RotationComponent,
    VelocityComponent,
);

pub type SuiteColumns = (
    Vec<TransformComponent>,
    Vec<PositionComponent>,
    Vec<RotationComponent>,
    Vec<VelocityComponent>,
);

pub fn suite_transform() -> TransformComponent {
    TransformComponent(BenchmarkMatrix::identity())
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

pub fn heavy_matrix() -> BenchmarkMatrix {
    BenchmarkMatrix::benchmark_rotation_x()
}

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

pub fn suite_columns(count: usize) -> SuiteColumns {
    (
        vec![suite_transform(); count],
        vec![suite_position(); count],
        vec![suite_rotation(); count],
        vec![suite_velocity(); count],
    )
}

pub fn suite_columns_from_bundles(bundles: &[SuiteBundle]) -> SuiteColumns {
    let mut transforms = Vec::with_capacity(bundles.len());
    let mut positions = Vec::with_capacity(bundles.len());
    let mut rotations = Vec::with_capacity(bundles.len());
    let mut velocities = Vec::with_capacity(bundles.len());
    for &(transform, position, rotation, velocity) in bundles {
        transforms.push(transform);
        positions.push(position);
        rotations.push(rotation);
        velocities.push(velocity);
    }
    (transforms, positions, rotations, velocities)
}

pub fn suite_columns_into_bundles(columns: SuiteColumns) -> Vec<SuiteBundle> {
    let (transforms, positions, rotations, velocities) = columns;
    transforms
        .into_iter()
        .zip(positions)
        .zip(rotations)
        .zip(velocities)
        .map(|(((transform, position), rotation), velocity)| {
            (transform, position, rotation, velocity)
        })
        .collect()
}

/// Construction-contract input with a different value in every row and column.
///
/// The benchmark intentionally uses identical values, but validation needs
/// distinct rows to catch adapters that reorder component ids without applying
/// the same permutation to their data columns.
pub fn distinct_suite_bundles(count: usize) -> Vec<SuiteBundle> {
    (0..count)
        .map(|index| {
            let value = index as f32 + 1.0;
            (
                TransformComponent(BenchmarkMatrix::from_scale(value)),
                PositionComponent(Vector3::new(value + 10.0, value + 11.0, value + 12.0)),
                RotationComponent(Vector3::new(value + 20.0, value + 21.0, value + 22.0)),
                VelocityComponent(Vector3::new(value + 30.0, value + 31.0, value + 32.0)),
            )
        })
        .collect()
}

pub fn distinct_suite_columns(count: usize) -> SuiteColumns {
    suite_columns_from_bundles(&distinct_suite_bundles(count))
}

pub fn assert_suite_bundles_match(actual: &mut [SuiteBundle], expected: &[SuiteBundle]) {
    actual.sort_by(|(_, left, _, _), (_, right, _, _)| left.0.x.total_cmp(&right.0.x));
    let mut expected = expected.to_vec();
    expected.sort_by(|(_, left, _, _), (_, right, _, _)| left.0.x.total_cmp(&right.0.x));
    assert_eq!(actual.len(), expected.len());
    for (
        (transform, position, rotation, velocity),
        (expected_transform, expected_position, expected_rotation, expected_velocity),
    ) in actual.iter().zip(expected)
    {
        assert_eq!(transform.0, expected_transform.0);
        assert_eq!(position.0, expected_position.0);
        assert_eq!(rotation.0, expected_rotation.0);
        assert_eq!(velocity.0, expected_velocity.0);
    }
}

pub fn light_bundle() -> (PositionComponent, VelocityComponent) {
    (suite_position(), suite_velocity())
}

pub fn random_access_bundle(index: usize) -> (PositionComponent, VelocityComponent) {
    let value = index as f32 + 1.0;
    (
        PositionComponent(Vector3::new(value, value * 0.5, value * 0.25)),
        suite_velocity(),
    )
}

pub fn assert_random_access_position(position: &PositionComponent, index: usize) {
    let expected = random_access_bundle(index).0;
    assert_eq!(position.0.x.to_bits(), expected.0.x.to_bits());
    assert_eq!(position.0.y.to_bits(), expected.0.y.to_bits());
    assert_eq!(position.0.z.to_bits(), expected.0.z.to_bits());
}

pub fn validate_random_access_order<T: Copy + Eq>(
    source_entities: &[T],
    order: &[T],
    mut lookup: impl FnMut(T) -> PositionComponent,
) {
    assert_eq!(source_entities.len(), order.len());
    for &entity in order {
        let index = source_entities
            .iter()
            .position(|&candidate| candidate == entity)
            .expect("random-access order contains an unknown entity");
        assert_random_access_position(&lookup(entity), index);
    }
}

pub fn heavy_bundle() -> SuiteBundle {
    (
        TransformComponent(heavy_matrix()),
        PositionComponent(Vector3::new(1.0, 2.0, 3.0)),
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
        PositionComponent(Vector3::new(1.0, 2.0, 3.0)),
        VelocityComponent(Vector3::new(0.5, 0.0, 0.5)),
    )
}

#[cfg(test)]
mod random_access_tests {
    use super::*;

    #[test]
    #[should_panic]
    fn repeated_first_entity_fails_identity_validation() {
        let entities = [0_usize, 1, 2, 3];
        validate_random_access_order(&entities, &entities, |_| random_access_bundle(0).0);
    }
}

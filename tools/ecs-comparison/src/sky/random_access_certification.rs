use super::random_access::{accessor_checksum, prepared_checksum, random_world_and_orders};
use crate::common::{position_checksum_value, PositionComponent};
use std::hint::black_box;
use std::time::{Duration, Instant};

const ACCESSOR_NAME: &str = "EntityAccessor::get";
const PREPARED_NAME: &str = "PreparedEntityAccess::iter";

#[derive(Debug, serde::Serialize)]
pub struct SkyRandomAccessCandidateResult {
    pub entity_count: usize,
    pub rotations: usize,
    pub traversals_per_rotation: usize,
    pub accessor_median_ns_per_traversal: f64,
    pub prepared_median_ns_per_traversal: f64,
    pub prepared_improvement_percent: f64,
    pub prepared_winning_rotations: usize,
    pub prepared_to_accessor_ratios: Vec<f64>,
    pub winner: &'static str,
}

#[derive(Debug, serde::Serialize)]
pub struct SkyRandomAccessApiCertification {
    pub rotations: usize,
    pub workloads: Vec<SkyRandomAccessCandidateResult>,
    pub geometric_mean_improvement_percent: f64,
    pub passed: bool,
}

pub fn certify_random_access_apis(rotations: usize) -> SkyRandomAccessApiCertification {
    assert!(rotations > 0);
    let workloads = [(10_000, 16_384), (100_000, 1_024)];
    let results: Vec<_> = workloads
        .into_iter()
        .map(|(entity_count, traversals)| certify_workload(entity_count, rotations, traversals))
        .collect();

    let geometric_mean_ratio = (results
        .iter()
        .map(|result| {
            result.prepared_median_ns_per_traversal / result.accessor_median_ns_per_traversal
        })
        .map(f64::ln)
        .sum::<f64>()
        / results.len() as f64)
        .exp();
    let required_wins = rotations.saturating_sub(1);
    let passed = results.iter().all(|result| {
        result.prepared_winning_rotations >= required_wins
            && result.prepared_median_ns_per_traversal
                <= result.accessor_median_ns_per_traversal * 1.015
    }) && geometric_mean_ratio <= 1.0 / 1.02;

    SkyRandomAccessApiCertification {
        rotations,
        workloads: results,
        geometric_mean_improvement_percent: (1.0 / geometric_mean_ratio - 1.0) * 100.0,
        passed,
    }
}

fn certify_workload(
    entity_count: usize,
    rotations: usize,
    traversals_per_rotation: usize,
) -> SkyRandomAccessCandidateResult {
    let (accessor_world, accessor_orders) = random_world_and_orders(entity_count);
    let accessor = accessor_world.accessor::<PositionComponent>();
    let (prepared_world, prepared_orders) = random_world_and_orders(entity_count);
    let plans: Vec<_> = prepared_orders
        .iter()
        .map(|entities| {
            prepared_world
                .prepare_access::<PositionComponent>(entities)
                .expect("certification fixture must be fully valid")
        })
        .collect();
    let expected = position_checksum_value(1.0, entity_count);
    let mut accessor_order = 0;
    let mut prepared_order = 0;

    for _ in 0..2 {
        assert_eq!(
            run_accessor(&accessor, &accessor_orders, &mut accessor_order),
            expected
        );
        assert_eq!(run_prepared(&plans, &mut prepared_order), expected);
    }

    let mut accessor_samples = Vec::with_capacity(rotations);
    let mut prepared_samples = Vec::with_capacity(rotations);
    let mut ratios = Vec::with_capacity(rotations);
    for rotation in 0..rotations {
        let (accessor_duration, prepared_duration) = if rotation % 2 == 0 {
            (
                time_traversals(traversals_per_rotation, || {
                    run_accessor(&accessor, &accessor_orders, &mut accessor_order)
                }),
                time_traversals(traversals_per_rotation, || {
                    run_prepared(&plans, &mut prepared_order)
                }),
            )
        } else {
            let prepared_duration = time_traversals(traversals_per_rotation, || {
                run_prepared(&plans, &mut prepared_order)
            });
            let accessor_duration = time_traversals(traversals_per_rotation, || {
                run_accessor(&accessor, &accessor_orders, &mut accessor_order)
            });
            (accessor_duration, prepared_duration)
        };

        let divisor = traversals_per_rotation as f64;
        accessor_samples.push(accessor_duration.as_nanos() as f64 / divisor);
        prepared_samples.push(prepared_duration.as_nanos() as f64 / divisor);
        ratios.push(prepared_duration.as_secs_f64() / accessor_duration.as_secs_f64());
    }

    let accessor_median = median(&mut accessor_samples);
    let prepared_median = median(&mut prepared_samples);
    SkyRandomAccessCandidateResult {
        entity_count,
        rotations,
        traversals_per_rotation,
        accessor_median_ns_per_traversal: accessor_median,
        prepared_median_ns_per_traversal: prepared_median,
        prepared_improvement_percent: (accessor_median / prepared_median - 1.0) * 100.0,
        prepared_winning_rotations: ratios.iter().filter(|&&ratio| ratio < 1.0).count(),
        prepared_to_accessor_ratios: ratios,
        winner: if prepared_median < accessor_median {
            PREPARED_NAME
        } else {
            ACCESSOR_NAME
        },
    }
}

fn time_traversals(traversals: usize, mut operation: impl FnMut() -> u64) -> Duration {
    let start = Instant::now();
    for _ in 0..traversals {
        black_box(operation());
    }
    start.elapsed()
}

fn run_accessor(
    accessor: &sky_ecs::EntityAccessor<'_, PositionComponent>,
    orders: &[Vec<sky_ecs::EntityId>],
    next_order: &mut usize,
) -> u64 {
    let checksum = accessor_checksum(accessor, &orders[*next_order % orders.len()]);
    *next_order += 1;
    checksum
}

fn run_prepared(
    plans: &[sky_ecs::PreparedEntityAccess<'_, PositionComponent>],
    next_order: &mut usize,
) -> u64 {
    let checksum = prepared_checksum(&plans[*next_order % plans.len()]);
    *next_order += 1;
    checksum
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    let middle = samples.len() / 2;
    if samples.len().is_multiple_of(2) {
        (samples[middle - 1] + samples[middle]) * 0.5
    } else {
        samples[middle]
    }
}

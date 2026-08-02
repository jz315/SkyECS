//! Local, feature-gated experiments for native parallel ECS execution.
//!
//! This module is deliberately absent from the formal 37-row comparison. One
//! process uses one fixed worker count so every adapter observes the same pool
//! width. Run separate processes to measure scaling.

use crate::common::{DataComponent, PositionComponent, RotationComponent, VelocityComponent};
use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
use cgmath::Vector3;
use criterion::Criterion;
use std::env;

pub const DENSE_ENTITY_COUNT: usize = 1_048_576;
pub const COMPUTE_ENTITY_COUNT: usize = 262_144;
pub const FRAGMENT_SHAPES: usize = 64;
pub const ENTITIES_PER_FRAGMENT: usize = 1_024;
pub const FRAGMENTED_ENTITY_COUNT: usize = FRAGMENT_SHAPES * ENTITIES_PER_FRAGMENT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParallelWorkload {
    DenseBandwidth,
    DenseCompute,
    FragmentedBandwidth,
}

impl ParallelWorkload {
    pub const ALL: [Self; 3] = [
        Self::DenseBandwidth,
        Self::DenseCompute,
        Self::FragmentedBandwidth,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::DenseBandwidth => "dense_bandwidth",
            Self::DenseCompute => "dense_compute",
            Self::FragmentedBandwidth => "fragmented_bandwidth",
        }
    }

    pub const fn entity_count(self) -> usize {
        match self {
            Self::DenseBandwidth => DENSE_ENTITY_COUNT,
            Self::DenseCompute => COMPUTE_ENTITY_COUNT,
            Self::FragmentedBandwidth => FRAGMENTED_ENTITY_COUNT,
        }
    }
}

pub struct ParallelRuntime {
    threads: usize,
}

impl ParallelRuntime {
    pub fn from_environment() -> Self {
        let available = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        let threads = env::var("SKY_ECS_PAR_THREADS")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .expect("SKY_ECS_PAR_THREADS must be a positive integer")
            })
            .unwrap_or_else(|| available.min(4));
        assert!(threads > 0, "SKY_ECS_PAR_THREADS must be positive");

        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("compare-ecs-par-{index}"))
            .build_global()
            .expect("parallel comparison must initialize Rayon before another global pool");
        rayon::scope(|_| {});

        ComputeTaskPool::get_or_init(|| {
            TaskPoolBuilder::new()
                .num_threads(threads)
                .thread_name("compare-ecs-bevy".to_owned())
                .build()
        });

        Self { threads }
    }

    pub const fn threads(&self) -> usize {
        self.threads
    }
}

pub fn parallel_bundle(
    logical_index: usize,
) -> (
    PositionComponent,
    VelocityComponent,
    RotationComponent,
    DataComponent,
) {
    let base = logical_index as f32;
    (
        PositionComponent(Vector3::new(base, base * 0.5, base * 0.25)),
        VelocityComponent(Vector3::new(1.0, 2.0, 3.0)),
        RotationComponent(Vector3::new(0.25, 0.5, 0.75)),
        DataComponent(0.125),
    )
}

#[inline]
pub fn position_checksum_value(position: &PositionComponent) -> u64 {
    let mut hash = (position.0.x.to_bits() as u64) ^ 0x9e37_79b9_7f4a_7c15;
    hash = (hash ^ position.0.y.to_bits() as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash = (hash ^ position.0.z.to_bits() as u64).wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^ (hash >> 31)
}

#[inline(always)]
pub fn bandwidth_kernel(position: &mut PositionComponent, velocity: &VelocityComponent) {
    position.0 += velocity.0;
}

#[inline(always)]
pub fn compute_kernel(
    position: &mut PositionComponent,
    velocity: &VelocityComponent,
    rotation: &RotationComponent,
    data: &DataComponent,
) {
    let mut x = position.0.x;
    let mut y = position.0.y;
    let mut z = position.0.z;
    for _ in 0..8 {
        x = (x + velocity.0.x * 0.25 + rotation.0.x * data.0) * 0.999_5;
        y = (y + velocity.0.y * 0.25 + rotation.0.y * data.0) * 0.999_5;
        z = (z + velocity.0.z * 0.25 + rotation.0.z * data.0) * 0.999_5;
    }
    position.0 = Vector3::new(x, y, z);
}

pub fn expected_position_checksum(workload: ParallelWorkload) -> u64 {
    (0..workload.entity_count()).fold(0_u64, |checksum, index| {
        let (mut position, velocity, rotation, data) = parallel_bundle(index);
        match workload {
            ParallelWorkload::DenseCompute => {
                compute_kernel(&mut position, &velocity, &rotation, &data)
            }
            ParallelWorkload::DenseBandwidth | ParallelWorkload::FragmentedBandwidth => {
                bandwidth_kernel(&mut position, &velocity)
            }
        }
        checksum.wrapping_add(position_checksum_value(&position))
    })
}

pub fn bench_parallel_candidates(criterion: &mut Criterion) {
    let runtime = ParallelRuntime::from_environment();
    eprintln!(
        "Parallel candidate run: {} worker thread(s); hecs 0.11 = N/A (no native public parallel query API)",
        runtime.threads()
    );

    for workload in ParallelWorkload::ALL {
        let mut group = criterion.benchmark_group(format!("parallel_query/{}", workload.name()));
        group.warm_up_time(std::time::Duration::from_millis(500));
        group.measurement_time(std::time::Duration::from_secs(2));
        group.sample_size(20);
        group.throughput(criterion::Throughput::Elements(
            workload.entity_count() as u64
        ));

        crate::sky::bench_parallel_query(&mut group, workload, &runtime);
        crate::bevy::bench_parallel_query(&mut group, workload, &runtime);
        crate::flecs_c::bench_parallel_query(&mut group, workload, &runtime);
        crate::freecs::bench_parallel_query(&mut group, workload, &runtime);
        crate::shipyard::bench_parallel_query(&mut group, workload, &runtime);
        group.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_workloads_have_stable_sizes_and_expected_values() {
        assert_eq!(ParallelWorkload::ALL.len(), 3);
        assert_eq!(FRAGMENTED_ENTITY_COUNT, 65_536);
        for workload in ParallelWorkload::ALL {
            assert!(workload.entity_count() > 0);
            assert_ne!(expected_position_checksum(workload), 0);
        }
    }
}

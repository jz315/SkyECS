use crate::common::*;
use bevy_ecs::entity::Entity as BevyEntity;
use bevy_ecs::world::World;
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;

mod dense_iteration;
mod entity_insertion;
mod fragmented_iteration;
mod gameplay_frame;
mod heavy_compute;
mod mixed_frame;
#[cfg(feature = "parallel-experiments")]
mod parallel;
mod random_access;
mod random_fragmented_iteration;
mod structural_changes;
mod validation;

pub use dense_iteration::{bench_iteration, bench_iteration_1m, bench_iteration_large};
pub use entity_insertion::{bench_bulk_construction, bench_single_insert};
pub use fragmented_iteration::bench_fragmented_iteration;
pub use gameplay_frame::{bench_gameplay_frame, bench_gameplay_phases, validate_gameplay_contract};
pub use heavy_compute::bench_heavy_compute;
pub use mixed_frame::{bench_mixed_frame, bench_mixed_frame_phases};
#[cfg(feature = "parallel-experiments")]
pub use parallel::bench_parallel_query;
pub use random_access::{bench_entity_id_random_access, bench_fixed_sequence_access};
pub use random_fragmented_iteration::bench_random_fragmented_iteration;
pub use structural_changes::bench_entity_ops;
pub use validation::validate_contract;

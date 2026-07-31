use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use sky_ecs::dynamic::{DynamicBundle, WorldDynamicExt};
use sky_ecs::{Bundle, EntityId, PreparedQuery, World};
use std::hint::black_box;

mod dense_iteration;
mod entity_insertion;
mod fragmented_iteration;
mod gameplay_frame;
mod heavy_compute;
#[cfg(feature = "api-experiments")]
mod heavy_compute_candidates;
mod mixed_frame;
mod random_access;
mod random_fragmented_iteration;
mod structural_changes;
mod validation;

pub use dense_iteration::{bench_iteration, bench_iteration_1m, bench_iteration_large};
pub use entity_insertion::{bench_bulk_construction, bench_single_insert};
pub use fragmented_iteration::bench_fragmented_iteration;
pub use gameplay_frame::{bench_gameplay_frame, bench_gameplay_phases, validate_gameplay_contract};
#[cfg(feature = "api-experiments")]
pub use gameplay_frame::{
    measure_ai_candidate, measure_frame_candidate, measure_iteration_candidate,
    measure_position_candidate, AiCandidate, FrameCandidateSelection, IterationCandidate,
    PositionCandidate,
};
pub use heavy_compute::bench_heavy_compute;
#[cfg(feature = "api-experiments")]
pub use heavy_compute_candidates::bench_heavy_compute_candidates;
pub use mixed_frame::{bench_mixed_frame, bench_mixed_frame_phases};
pub use random_access::{bench_entity_id_random_access, bench_fixed_sequence_access};
pub use random_fragmented_iteration::bench_random_fragmented_iteration;
pub use structural_changes::bench_entity_ops;
pub use validation::validate_contract;

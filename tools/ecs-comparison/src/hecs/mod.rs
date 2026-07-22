use crate::common::*;
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use hecs::{Entity as HecsEntity, PreparedQuery, Query as HecsQuery, World};
use std::hint::black_box;

mod dense_iteration;
mod entity_insertion;
mod fragmented_iteration;
mod gameplay_frame;
mod heavy_compute;
mod mixed_frame;
mod random_access;
mod random_fragmented_iteration;
mod structural_changes;
mod validation;

pub use dense_iteration::{bench_iteration, bench_iteration_1m, bench_iteration_large};
pub use entity_insertion::{bench_native_bulk, bench_single_insert};
pub use fragmented_iteration::bench_fragmented_iteration;
pub use gameplay_frame::{bench_gameplay_frame, bench_gameplay_phases, validate_gameplay_contract};
pub use heavy_compute::bench_heavy_compute;
pub use mixed_frame::{bench_mixed_frame, bench_mixed_frame_phases};
pub use random_access::{bench_entity_id_random_access, bench_fixed_sequence_access};
pub use random_fragmented_iteration::bench_random_fragmented_iteration;
pub use structural_changes::bench_entity_ops;
pub use validation::validate_contract;

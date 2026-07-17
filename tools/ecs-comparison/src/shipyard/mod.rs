use crate::common::*;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use shipyard::{EntityId, Get, IntoIter, View, ViewMut, World};
use std::hint::black_box;

mod dense_iteration;
mod entity_insertion;
mod fragmented_iteration;
mod heavy_compute;
mod mixed_frame;
mod random_access;
mod random_fragmented_iteration;
mod structural_changes;
mod validation;

pub use dense_iteration::{bench_iteration, bench_iteration_1m, bench_iteration_large};
pub use entity_insertion::bench_insert;
pub use fragmented_iteration::bench_fragmented_iteration;
pub use heavy_compute::bench_heavy_compute;
pub use mixed_frame::{bench_mixed_frame, bench_mixed_frame_phases};
pub use random_access::bench_random_access;
pub use random_fragmented_iteration::bench_random_fragmented_iteration;
pub use structural_changes::bench_entity_ops;
pub use validation::validate_contract;

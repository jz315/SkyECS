use criterion::{measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode};
use sky_ecs_comparison::{bevy, engine_order, flecs_c, freecs, hecs, shipyard, sky, Engine};
use std::time::Duration;

macro_rules! dispatch {
    ($engine:expr, $group:expr) => {
        match $engine {
            Engine::Sky => sky::bench_fixed_sequence_access($group),
            Engine::Hecs => hecs::bench_fixed_sequence_access($group),
            Engine::Bevy => bevy::bench_fixed_sequence_access($group),
            Engine::FlecsC => flecs_c::bench_fixed_sequence_access($group),
            Engine::Freecs => freecs::bench_fixed_sequence_access($group),
            Engine::Shipyard => shipyard::bench_fixed_sequence_access($group),
        }
    };
}

pub(crate) fn run(criterion: &mut Criterion) {
    let mut group: BenchmarkGroup<'_, WallTime> =
        criterion.benchmark_group("fixed_sequence_access");
    group
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
        .sample_size(30);
    for engine in engine_order() {
        dispatch!(engine, &mut group);
    }
    group.finish();
}

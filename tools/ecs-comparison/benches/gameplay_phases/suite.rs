use criterion::{measurement::WallTime, Criterion};
use sky_ecs_comparison::{bevy, engine_order, flecs_c, freecs, hecs, shipyard, sky, Engine};
use std::time::Duration;

macro_rules! dispatch {
    ($engine:expr, $group:expr) => {
        match $engine {
            Engine::Sky => sky::bench_gameplay_phases($group),
            Engine::Hecs => hecs::bench_gameplay_phases($group),
            Engine::Bevy => bevy::bench_gameplay_phases($group),
            Engine::FlecsC => flecs_c::bench_gameplay_phases($group),
            Engine::Freecs => freecs::bench_gameplay_phases($group),
            Engine::Shipyard => shipyard::bench_gameplay_phases($group),
        }
    };
}

pub(crate) fn bench_gameplay_phases(criterion: &mut Criterion) {
    let mut group: criterion::BenchmarkGroup<'_, WallTime> =
        criterion.benchmark_group("gameplay_scenario");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
        .sample_size(30);
    for engine in engine_order() {
        dispatch!(engine, &mut group);
    }
    group.finish();
}

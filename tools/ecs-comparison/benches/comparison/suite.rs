#![allow(clippy::too_many_arguments)]

use criterion::{measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode};
use sky_ecs_comparison::{bevy, engine_order, flecs_c, freecs, hecs, shipyard, sky, Engine};
use std::time::Duration;

macro_rules! dispatch {
    ($engine:expr, $function:ident, $group:expr) => {
        match $engine {
            Engine::Sky => sky::$function($group),
            Engine::Hecs => hecs::$function($group),
            Engine::Bevy => bevy::$function($group),
            Engine::FlecsC => flecs_c::$function($group),
            Engine::Freecs => freecs::$function($group),
            Engine::Shipyard => shipyard::$function($group),
        }
    };
}

fn benchmark_group<'a>(c: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    group
}

pub(crate) fn bench_insert(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_construction");
    for engine in engine_order() {
        dispatch!(engine, bench_insert, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_iteration(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_iteration");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_iteration_large(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_iteration_large");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration_large, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_iteration_1m(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_iteration_1m");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration_1m, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_fragmented_iteration(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_fragmented_iteration");
    for engine in engine_order() {
        dispatch!(engine, bench_fragmented_iteration, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_random_fragmented_iteration(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_random_fragmented_iteration");
    for engine in engine_order() {
        dispatch!(engine, bench_random_fragmented_iteration, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_heavy_compute(c: &mut Criterion) {
    let mut group = benchmark_group(c, "diagnostic_heavy_compute");
    for engine in engine_order() {
        dispatch!(engine, bench_heavy_compute, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_random_access(c: &mut Criterion) {
    let mut group = benchmark_group(c, "prepared_random_access");
    group.sampling_mode(SamplingMode::Flat);
    for engine in engine_order() {
        dispatch!(engine, bench_random_access, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_entity_ops(c: &mut Criterion) {
    let mut group = benchmark_group(c, "entity_ops");
    for engine in engine_order() {
        dispatch!(engine, bench_entity_ops, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_mixed_frame(c: &mut Criterion) {
    let mut group = benchmark_group(c, "scenario_mixed_frame");
    for engine in engine_order() {
        dispatch!(engine, bench_mixed_frame, &mut group);
    }
    group.finish();
}

pub(crate) fn bench_mixed_frame_phases(c: &mut Criterion) {
    let mut group = benchmark_group(c, "scenario_mixed_frame_phases");
    for engine in engine_order() {
        dispatch!(engine, bench_mixed_frame_phases, &mut group);
    }
    group.finish();
}

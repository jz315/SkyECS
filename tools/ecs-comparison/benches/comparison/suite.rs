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

pub(crate) fn bench_construction(c: &mut Criterion) {
    let mut group = benchmark_group(c, "entity_construction");
    for engine in engine_order() {
        dispatch!(engine, bench_single_insert, &mut group);
        dispatch!(engine, bench_native_bulk, &mut group);
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
    let mut group = benchmark_group(c, "random_fragmentation");
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

pub(crate) fn bench_entity_id_random_access(c: &mut Criterion) {
    let mut group = benchmark_group(c, "entity_id_random_access");
    group.sampling_mode(SamplingMode::Flat);
    for engine in engine_order() {
        dispatch!(engine, bench_entity_id_random_access, &mut group);
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

pub(crate) fn bench_gameplay_scenario(c: &mut Criterion) {
    let mut group = benchmark_group(c, "gameplay_scenario");
    for engine in engine_order() {
        dispatch!(engine, bench_gameplay_frame, &mut group);
    }
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
        .sample_size(30);
    for engine in engine_order() {
        dispatch!(engine, bench_gameplay_phases, &mut group);
    }
    group.finish();
}

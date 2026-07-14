#![allow(clippy::too_many_arguments)]

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use sky_ecs_comparison::{
    bevy, engine_order, flecs, flecs_cpp, freecs, hecs, shipyard, sky, Engine,
};
use std::time::Duration;

macro_rules! dispatch {
    ($engine:expr, $function:ident, $group:expr) => {
        match $engine {
            Engine::Sky => sky::$function($group),
            Engine::Hecs => hecs::$function($group),
            Engine::Bevy => bevy::$function($group),
            Engine::Flecs => flecs::$function($group),
            Engine::FlecsCpp => flecs_cpp::$function($group),
            Engine::Freecs => freecs::$function($group),
            Engine::Shipyard => shipyard::$function($group),
        }
    };
}

fn fair_group<'a>(c: &'a mut Criterion, name: &str) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    group
}

fn bench_insert(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_construction");
    for engine in engine_order() {
        dispatch!(engine, bench_insert, &mut group);
    }
    group.finish();
}

fn bench_iteration(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_prepared_iteration");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration, &mut group);
    }
    group.finish();
}

fn bench_iteration_repeated(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_prepared_iteration_repeated");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration_repeated, &mut group);
    }
    group.finish();
}

fn bench_iteration_large(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_prepared_iteration_large");
    for engine in engine_order() {
        dispatch!(engine, bench_iteration_large, &mut group);
    }
    group.finish();
}

fn bench_fragmented_iteration(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_prepared_fragmented_iteration");
    for engine in engine_order() {
        dispatch!(engine, bench_fragmented_iteration, &mut group);
    }
    group.finish();
}

fn bench_heavy_compute(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_diagnostic_heavy_compute");
    for engine in engine_order() {
        dispatch!(engine, bench_heavy_compute, &mut group);
    }
    group.finish();
}

fn bench_random_access(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_prepared_random_access");
    for engine in engine_order() {
        dispatch!(engine, bench_random_access, &mut group);
    }
    group.finish();
}

fn bench_entity_ops(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_entity_ops");
    for engine in engine_order() {
        dispatch!(engine, bench_entity_ops, &mut group);
    }
    group.finish();
}

fn bench_flecs_spawn_despawn_modes(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_diagnostic_flecs_spawn_despawn");
    flecs::bench_spawn_despawn_modes(&mut group);
    group.finish();
}

fn bench_mixed_frame(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_scenario_mixed_frame");
    for engine in engine_order() {
        dispatch!(engine, bench_mixed_frame, &mut group);
    }
    group.finish();
}

fn bench_mixed_frame_phases(c: &mut Criterion) {
    let mut group = fair_group(c, "fair_scenario_mixed_frame_phases");
    for engine in engine_order() {
        dispatch!(engine, bench_mixed_frame_phases, &mut group);
    }
    group.finish();
}

criterion_group!(
    fair_benches,
    bench_insert,
    bench_iteration,
    bench_iteration_repeated,
    bench_iteration_large,
    bench_fragmented_iteration,
    bench_heavy_compute,
    bench_random_access,
    bench_entity_ops,
    bench_flecs_spawn_despawn_modes,
    bench_mixed_frame,
    bench_mixed_frame_phases,
);
criterion_main!(fair_benches);

use super::native::*;
use super::*;

fn random_context(entity_count: usize) -> Context {
    // SAFETY: Registered workloads always use a non-zero count.
    unsafe {
        Context::new(
            sky_flecs_c_random_new(entity_count),
            sky_flecs_c_random_delete,
        )
    }
}
pub fn bench_entity_id_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, entity_count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        let mut context = None;
        group.bench_function(format!("{name}/flecs_c"), move |bencher| {
            let context = context.get_or_insert_with(|| random_context(entity_count));
            bencher.iter(|| {
                // SAFETY: The prepared context remains alive for the timed loop.
                black_box(unsafe { sky_flecs_c_random_run(context.pointer()) });
            });
        });
    }
}

pub fn bench_fixed_sequence_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (label, entity_count) in [
        ("10k", SIMPLE_ENTITY_COUNT),
        ("100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("build_{label}/flecs_c"), |bencher| {
            let context = random_context(entity_count);
            bencher.iter(|| {
                // SAFETY: context is alive and zero repeats requests plan
                // construction without a component traversal.
                black_box(unsafe { sky_flecs_c_fixed_sequence_build_run(context.pointer(), 0) });
            });
        });

        group.bench_function(format!("steady_{label}/flecs_c"), |bencher| {
            let context = random_context(entity_count);
            bencher.iter(|| {
                // SAFETY: context owns structurally stable pointer plans.
                black_box(unsafe { sky_flecs_c_fixed_sequence_steady_run(context.pointer()) });
            });
        });

        for repeats in [1_usize, 4, 16, 64] {
            group.bench_function(format!("amortized_{label}_x{repeats}/flecs_c"), |bencher| {
                let context = random_context(entity_count);
                bencher.iter(|| {
                    // SAFETY: context is alive; the native call builds a
                    // fresh plan and traverses it exactly `repeats` times.
                    black_box(unsafe {
                        sky_flecs_c_fixed_sequence_build_run(context.pointer(), repeats)
                    });
                });
            });
        }
    }
}

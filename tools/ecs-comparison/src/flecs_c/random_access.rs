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
pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
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

use super::native::*;
use super::*;

fn simple_context(entity_count: usize) -> Context {
    // SAFETY: The constructor consumes the count before returning owned state.
    unsafe {
        Context::new(
            sky_flecs_c_simple_new(entity_count),
            sky_flecs_c_simple_delete,
        )
    }
}
fn bench_simple_iteration(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    entity_count: usize,
    repetitions: usize,
) {
    let mut context = None;
    group.bench_function(name, move |bencher| {
        let context = context.get_or_insert_with(|| simple_context(entity_count));
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_simple_run(context.pointer(), repetitions) });
        });
    });
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    bench_simple_iteration(group, "simple_10k/flecs_c", SIMPLE_ENTITY_COUNT, 1);
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    bench_simple_iteration(
        group,
        "simple_100k/flecs_c",
        LARGE_ITERATION_ENTITY_COUNT,
        1,
    );
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    bench_simple_iteration(
        group,
        "simple_1m/flecs_c",
        VERY_LARGE_ITERATION_ENTITY_COUNT,
        1,
    );
}

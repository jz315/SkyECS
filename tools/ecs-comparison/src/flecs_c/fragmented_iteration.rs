use super::native::*;
use super::*;

fn fragmented_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_fragmented_new(), sky_flecs_c_fragmented_delete) }
}
pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = None;
    group.bench_function("fragmented_26x400/flecs_c", move |bencher| {
        let context = context.get_or_insert_with(fragmented_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_fragmented_run(context.pointer()) });
        });
    });
}

use super::native::*;
use super::*;

fn heavy_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_heavy_new(), sky_flecs_c_heavy_delete) }
}
pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = None;
    group.bench_function("heavy/flecs_c", move |bencher| {
        let context = context.get_or_insert_with(heavy_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_heavy_run(context.pointer()) });
        });
    });
}

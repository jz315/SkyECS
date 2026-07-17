use super::native::*;
use super::*;

fn mixed_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_mixed_new(), sky_flecs_c_mixed_delete) }
}
pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = None;
    group.bench_function("frame/flecs_c", move |bencher| {
        let context = context.get_or_insert_with(mixed_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_mixed_frame(context.pointer()) });
        });
    });
}

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    bench_mixed_phase(group, "movement/flecs_c", sky_flecs_c_mixed_movement);
    bench_mixed_phase(group, "health/flecs_c", sky_flecs_c_mixed_health);
    bench_mixed_phase(group, "heavy/flecs_c", sky_flecs_c_mixed_heavy);
    bench_mixed_phase(group, "random_access/flecs_c", sky_flecs_c_mixed_random);
    bench_mixed_phase(group, "structural_churn/flecs_c", sky_flecs_c_mixed_churn);
    bench_mixed_phase(group, "spawn_despawn/flecs_c", sky_flecs_c_mixed_spawn);
}

fn bench_mixed_phase(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    operation: unsafe extern "C" fn(*mut c_void) -> u64,
) {
    let mut context = None;
    group.bench_function(name, move |bencher| {
        let context = context.get_or_insert_with(mixed_context);
        bencher.iter(|| {
            // SAFETY: Every accepted operation is defined for a mixed context.
            black_box(unsafe { operation(context.pointer()) });
        });
    });
}

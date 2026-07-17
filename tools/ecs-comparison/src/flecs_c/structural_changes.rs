use super::native::*;
use super::*;

fn entity_ops_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_entity_ops_new(), sky_flecs_c_entity_ops_delete) }
}

fn add_remove_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_add_remove_new(), sky_flecs_c_add_remove_delete) }
}
pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut spawn_context = None;
    group.bench_function("spawn_despawn_1k/flecs_c", move |bencher| {
        let spawn_context = spawn_context.get_or_insert_with(entity_ops_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_spawn_despawn(spawn_context.pointer()) });
        });
    });

    let mut component_context = None;
    group.bench_function("add_remove_component_1k/flecs_c", move |bencher| {
        let component_context = component_context.get_or_insert_with(add_remove_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_add_remove(component_context.pointer()) });
        });
    });
}

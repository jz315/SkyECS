use super::native::*;
use super::*;

fn insert_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_insert_new(), sky_flecs_c_insert_delete) }
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/flecs_c", |bencher| {
        bencher.iter_batched_ref(
            insert_context,
            |context| {
                // SAFETY: `context` is the insertion context expected by this call.
                black_box(unsafe { sky_flecs_c_bulk_from_columns(context.pointer()) });
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn bench_single_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("single_insert_10k/flecs_c", |bencher| {
        bencher.iter_batched_ref(
            insert_context,
            |context| {
                // SAFETY: `context` is the insertion context expected by this call.
                black_box(unsafe { sky_flecs_c_single_insert(context.pointer()) });
            },
            BatchSize::SmallInput,
        );
    });
}

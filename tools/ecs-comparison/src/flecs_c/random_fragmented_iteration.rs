use super::native::*;
use super::*;

fn random_fragmented_context(
    storage: usize,
    component_count: usize,
    term_count: usize,
    masks: &[u16],
) -> Context {
    // SAFETY: Supported workload dimensions are supplied by the shared matrix.
    // The native constructor consumes `masks` synchronously and stores no pointer.
    unsafe {
        Context::new(
            sky_flecs_c_random_fragmented_new(
                storage,
                component_count,
                term_count,
                masks.as_ptr(),
                masks.len(),
            ),
            sky_flecs_c_random_fragmented_delete,
        )
    }
}
pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (storage, label) in [(0, "tags"), (1, "components")] {
        for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
            let suffix = if term_count == 1 { "term" } else { "terms" };
            group.bench_function(
                format!("random_{component_count}_{label}_{term_count}_{suffix}/flecs_c"),
                move |bencher| {
                    let masks = random_fragment_masks(component_count);
                    let expected = random_fragment_match_count(&masks, term_count) as u64;
                    let context =
                        random_fragmented_context(storage, component_count, term_count, &masks);
                    // SAFETY: The prepared context remains alive and has this concrete type.
                    assert_eq!(
                        unsafe { sky_flecs_c_random_fragmented_count(context.pointer()) },
                        expected
                    );
                    bencher.iter(|| {
                        // SAFETY: The prepared context remains alive for the timed loop.
                        unsafe { sky_flecs_c_random_fragmented_run(context.pointer()) }
                    });
                },
            );
        }
    }
}

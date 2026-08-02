use super::*;
use crate::parallel::{expected_position_checksum, ParallelRuntime, ParallelWorkload};
use native::{
    sky_flecs_c_parallel_checksum, sky_flecs_c_parallel_delete, sky_flecs_c_parallel_new,
    sky_flecs_c_parallel_run, Context,
};

const fn workload_id(workload: ParallelWorkload) -> u32 {
    match workload {
        ParallelWorkload::DenseBandwidth => 0,
        ParallelWorkload::DenseCompute => 1,
        ParallelWorkload::FragmentedBandwidth => 2,
    }
}

pub fn bench_parallel_query(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: ParallelWorkload,
    runtime: &ParallelRuntime,
) {
    group.bench_function(format!("threads_{}/flecs", runtime.threads()), |bencher| {
        let context = Context::new(
            // SAFETY: The workload discriminator and positive worker count
            // satisfy the native constructor contract.
            unsafe { sky_flecs_c_parallel_new(workload_id(workload), runtime.threads() as u32) },
            sky_flecs_c_parallel_delete,
        );
        // SAFETY: `context` owns a live native parallel context.
        unsafe { sky_flecs_c_parallel_run(context.pointer()) };
        // SAFETY: Checksum traversal is serial and the preceding progress call
        // has joined every Flecs worker.
        let checksum = unsafe { sky_flecs_c_parallel_checksum(context.pointer()) };
        assert_eq!(
            checksum,
            expected_position_checksum(workload),
            "Flecs parallel adapter must update every matching entity exactly once"
        );
        bencher.iter(|| {
            // SAFETY: Each call completes and joins the native pipeline before
            // returning; Criterion invokes the closure serially.
            black_box(unsafe { sky_flecs_c_parallel_run(context.pointer()) });
        });
    });
}

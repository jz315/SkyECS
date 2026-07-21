use super::native::*;
use crate::common::{GameplayDigest, GAMEPLAY_CANONICAL_DIGEST};
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;

fn gameplay_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_gameplay_new(), sky_flecs_c_gameplay_delete) }
}

pub fn validate_gameplay_contract() {
    let mut context = gameplay_context();
    let mut native = NativeGameplayDigest::default();
    // SAFETY: `context` and `native` remain alive for the call, and the native
    // adapter writes exactly one repr(C) digest value.
    assert!(unsafe { sky_flecs_c_gameplay_run_trace(context.pointer(), &mut native) });
    let digest = GameplayDigest {
        entity_count: native.entity_count as usize,
        moving_count: native.moving_count as usize,
        health_count: native.health_count as usize,
        lifetime_count: native.lifetime_count as usize,
        stunned_count: native.stunned_count as usize,
        position_checksum: native.position_checksum,
        health_checksum: native.health_checksum,
        lifetime_checksum: native.lifetime_checksum,
        generation_checksum: native.generation_checksum,
        ai_lookup_checksum: native.ai_lookup_checksum,
    };
    assert_eq!(digest, GAMEPLAY_CANONICAL_DIGEST);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = None;
    group.bench_function("frame/flecs_c", move |bencher| {
        let context = context.get_or_insert_with(gameplay_context);
        bencher.iter(|| {
            // SAFETY: The prepared context remains alive for the timed loop.
            black_box(unsafe { sky_flecs_c_gameplay_frame(context.pointer()) });
        });
    });
}

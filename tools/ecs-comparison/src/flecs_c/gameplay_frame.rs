use super::native::*;
use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;

fn gameplay_context() -> Context {
    // SAFETY: This constructor has no preconditions and returns owned state.
    unsafe { Context::new(sky_flecs_c_gameplay_new(), sky_flecs_c_gameplay_delete) }
}

struct FlecsGameplayWorld {
    context: Context,
}

impl FlecsGameplayWorld {
    fn new(_trace: &GameplayTrace) -> Self {
        Self {
            context: gameplay_context(),
        }
    }

    fn run_native_frame(&mut self, _frame: &GameplayFrame) {
        let checksum = unsafe {
            // SAFETY: the owned native context remains live for this single
            // full-frame FFI call.
            sky_flecs_c_gameplay_frame(self.context.pointer())
        };
        black_box(checksum);
    }
}

impl GameplayPhaseAdapter for FlecsGameplayWorld {
    fn run_phase(&mut self, phase: GameplayPhase, _frame: &GameplayFrame) {
        let checksum = unsafe {
            // SAFETY: every native phase function accepts the live owned
            // context and preserves it for the following phase.
            match phase {
                GameplayPhase::Iteration => sky_flecs_c_gameplay_iteration(self.context.pointer()),
                GameplayPhase::AiSourceLookup => {
                    sky_flecs_c_gameplay_ai_source(self.context.pointer())
                }
                GameplayPhase::TargetPositionLookup => {
                    sky_flecs_c_gameplay_target_positions(self.context.pointer())
                }
                GameplayPhase::StatusTransition => {
                    sky_flecs_c_gameplay_status_transition(self.context.pointer())
                }
                GameplayPhase::ProjectileRecycle => {
                    sky_flecs_c_gameplay_projectile_recycle(self.context.pointer())
                }
            }
        };
        black_box(checksum);
    }

    fn digest(&self) -> GameplayDigest {
        let mut native = NativeGameplayDigest::default();
        let context = self.context.pointer();
        let success = unsafe {
            // SAFETY: shared digest inspection does not mutate component state
            // and writes exactly one repr(C) output value.
            sky_flecs_c_gameplay_digest(context, &mut native)
        };
        assert!(success);
        native_digest(native)
    }
}

fn native_digest(native: NativeGameplayDigest) -> GameplayDigest {
    GameplayDigest {
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
    }
}

pub fn validate_gameplay_contract() {
    let context = gameplay_context();
    let mut native = NativeGameplayDigest::default();
    // SAFETY: `context` and `native` remain alive for the call, and the native
    // adapter writes exactly one repr(C) digest value.
    assert!(unsafe { sky_flecs_c_gameplay_run_trace(context.pointer(), &mut native) });
    let digest = native_digest(native);
    assert_eq!(digest, GAMEPLAY_CANONICAL_DIGEST);

    let trace = GameplayTrace::standard();
    let mut phased = FlecsGameplayWorld::new(&trace);
    for frame in trace.frames() {
        GameplayPhaseAdapter::run_frame(&mut phased, frame);
    }
    assert_eq!(
        GameplayPhaseAdapter::digest(&phased),
        GAMEPLAY_CANONICAL_DIGEST
    );
}

pub fn bench_gameplay_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_gameplay_phases(group, "flecs_c", FlecsGameplayWorld::new);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_full_gameplay_frames(
        group,
        "flecs_c",
        FlecsGameplayWorld::new,
        FlecsGameplayWorld::run_native_frame,
    );
}

use super::native::Context;
use super::native::*;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BulkConstructionCandidate {
    PreparedTable,
    ResolveTableFromIds,
    RemapInTimedPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpawnCandidate {
    TableGetMut,
    TableSetId,
    BulkOne,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddRemoveCandidate {
    SetId,
    Emplace,
    AddThenGetMut,
}

fn insert_context(candidate: BulkConstructionCandidate) -> Context {
    // SAFETY: the constructor has no preconditions and is paired with its
    // native deleter by Context.
    let pointer = match candidate {
        BulkConstructionCandidate::ResolveTableFromIds => unsafe {
            sky_flecs_c_insert_new_unprepared_table()
        },
        BulkConstructionCandidate::PreparedTable | BulkConstructionCandidate::RemapInTimedPath => unsafe {
            sky_flecs_c_insert_new()
        },
    };
    Context::new(pointer, sky_flecs_c_insert_delete)
}

fn spawn_context() -> Context {
    // SAFETY: the constructor has no preconditions and is paired with its
    // native deleter by Context.
    unsafe { Context::new(sky_flecs_c_entity_ops_new(), sky_flecs_c_entity_ops_delete) }
}

fn add_remove_context() -> Context {
    // SAFETY: the constructor has no preconditions and is paired with its
    // native deleter by Context.
    unsafe { Context::new(sky_flecs_c_add_remove_new(), sky_flecs_c_add_remove_delete) }
}

unsafe fn run_bulk(context: &Context, candidate: BulkConstructionCandidate) -> u64 {
    match candidate {
        BulkConstructionCandidate::PreparedTable => unsafe {
            sky_flecs_c_bulk_from_columns_prepared(context.pointer())
        },
        BulkConstructionCandidate::ResolveTableFromIds => unsafe {
            sky_flecs_c_bulk_from_columns_resolve_table(context.pointer())
        },
        BulkConstructionCandidate::RemapInTimedPath => unsafe {
            sky_flecs_c_bulk_from_columns_remap(context.pointer())
        },
    }
}

unsafe fn run_spawn(context: &Context, candidate: SpawnCandidate) -> u64 {
    match candidate {
        SpawnCandidate::TableGetMut => unsafe {
            sky_flecs_c_spawn_despawn_get_mut(context.pointer())
        },
        SpawnCandidate::TableSetId => unsafe {
            sky_flecs_c_spawn_despawn_set_id(context.pointer())
        },
        SpawnCandidate::BulkOne => unsafe { sky_flecs_c_spawn_despawn_bulk_one(context.pointer()) },
    }
}

unsafe fn run_add_remove(context: &Context, candidate: AddRemoveCandidate) -> u64 {
    match candidate {
        AddRemoveCandidate::SetId => unsafe { sky_flecs_c_add_remove_set_id(context.pointer()) },
        AddRemoveCandidate::Emplace => unsafe { sky_flecs_c_add_remove_emplace(context.pointer()) },
        AddRemoveCandidate::AddThenGetMut => unsafe {
            sky_flecs_c_add_remove_add_get_mut(context.pointer())
        },
    }
}

/// Measures independent empty-World bulk construction operations.
///
/// Context creation, component registration, source-vector generation and
/// destruction remain outside each timed interval, matching the canonical
/// construction boundary.
pub fn measure_bulk_candidate(
    candidate: BulkConstructionCandidate,
    repetitions: usize,
) -> Duration {
    let mut measured = Duration::ZERO;
    let mut remaining = repetitions;
    while remaining != 0 {
        // Keep the candidate's World and source-column construction wholly
        // outside timing. A bounded batch avoids retaining unbounded amounts
        // of the 1 MiB fixture when Criterion requests many iterations.
        let batch_len = remaining.min(16);
        let contexts: Vec<_> = (0..batch_len).map(|_| insert_context(candidate)).collect();
        let start = Instant::now();
        for context in &contexts {
            // SAFETY: the context and operation belong to the same native module.
            black_box(unsafe { run_bulk(context, candidate) });
        }
        measured += start.elapsed();
        drop(contexts);
        remaining -= batch_len;
    }
    measured
}

/// Measures steady-state spawn/despawn churn after one candidate-specific
/// warm-up cycle has populated Flecs' table and entity-ID reuse state.
pub fn measure_spawn_candidate(candidate: SpawnCandidate, repetitions: usize) -> Duration {
    let context = spawn_context();
    // SAFETY: the context and operation belong to the same native module.
    black_box(unsafe { run_spawn(&context, candidate) });
    let start = Instant::now();
    for _ in 0..repetitions {
        // SAFETY: every call returns the context to the same empty live-row
        // state and keeps the context alive for the complete loop.
        black_box(unsafe { run_spawn(&context, candidate) });
    }
    start.elapsed()
}

/// Measures steady-state component add/remove transitions after one
/// candidate-specific warm-up cycle has created the target table edge.
pub fn measure_add_remove_candidate(candidate: AddRemoveCandidate, repetitions: usize) -> Duration {
    let context = add_remove_context();
    // SAFETY: the context and operation belong to the same native module.
    black_box(unsafe { run_add_remove(&context, candidate) });
    let start = Instant::now();
    for _ in 0..repetitions {
        // SAFETY: every call removes all added components before returning.
        black_box(unsafe { run_add_remove(&context, candidate) });
    }
    start.elapsed()
}

use super::gameplay_frame::{
    SkyAiApi, SkyGameplayWorld, SkyIterationApi, SkyLookupApi, SELECTED_AI_API, SELECTED_LOOKUP_API,
};
use crate::common::{GameplayTrace, GAMEPLAY_FRAME_COUNT};
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;

pub fn bench_gameplay_api_candidates(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, iteration, ai, lookup) in [
        (
            "iteration_chunk_closure",
            SkyIterationApi::ChunkClosure,
            SELECTED_AI_API,
            SkyLookupApi::EntityAccessor,
        ),
        (
            "iteration_chunk_function",
            SkyIterationApi::ChunkFunction,
            SELECTED_AI_API,
            SkyLookupApi::EntityAccessor,
        ),
        (
            "lookup_world_get",
            SkyIterationApi::ChunkFunction,
            SELECTED_AI_API,
            SkyLookupApi::WorldGet,
        ),
        (
            "lookup_entity_accessor",
            SkyIterationApi::ChunkFunction,
            SELECTED_AI_API,
            SkyLookupApi::EntityAccessor,
        ),
        (
            "lookup_prepared_entity_view",
            SkyIterationApi::ChunkFunction,
            SELECTED_AI_API,
            SkyLookupApi::PreparedEntityView,
        ),
        (
            "ai_world_get_pair",
            SkyIterationApi::ChunkFunction,
            SkyAiApi::WorldGetPair,
            SELECTED_LOOKUP_API,
        ),
        (
            "ai_split_accessors",
            SkyIterationApi::ChunkFunction,
            SkyAiApi::SplitAccessors,
            SELECTED_LOOKUP_API,
        ),
        (
            "ai_prepared_entity_view",
            SkyIterationApi::ChunkFunction,
            SkyAiApi::PreparedEntityView,
            SELECTED_LOOKUP_API,
        ),
    ] {
        group.bench_function(name, move |bencher| {
            let trace = GameplayTrace::standard();
            let mut gameplay = SkyGameplayWorld::new(&trace);
            let mut frame = 0;
            bencher.iter(|| {
                gameplay.run_frame_with_apis(&trace.frames()[frame], iteration, ai, lookup);
                frame = (frame + 1) % GAMEPLAY_FRAME_COUNT;
                black_box(&gameplay);
            });
        });
    }
}

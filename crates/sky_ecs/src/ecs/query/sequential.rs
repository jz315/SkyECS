use super::{CachedArchetype, Chunk, ComponentIndexMap, EntityId, QuerySpec, World};
use crate::ecs::ChunkId;
use std::sync::Arc;

const MIN_FLATTENED_CHUNKS: usize = 8;
const MAX_AVERAGE_ENTITIES_PER_CHUNK: usize = 512;

#[derive(Clone, Copy)]
pub(crate) struct SequentialChunk {
    chunk_id: ChunkId,
    pub(crate) component_indices: ComponentIndexMap,
}

impl SequentialChunk {
    #[inline(always)]
    pub(crate) fn chunk<'w>(&self, world: &'w World) -> &'w Chunk {
        world.resolve_chunk(self.chunk_id)
    }
}

#[inline(always)]
pub(crate) fn run_for_each<Q, F>(world: &World, chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(Q::Item<'w>),
{
    for cached in chunks {
        unsafe {
            Q::for_each_entity(cached.chunk(world), &cached.component_indices, &mut f);
        }
    }
}

#[inline(always)]
pub(crate) fn run_for_each_with_entity<Q, F>(world: &World, chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(EntityId, Q::Item<'w>),
{
    for cached in chunks {
        unsafe {
            let chunk = cached.chunk(world);
            let entities = chunk.entities();
            let mut entity_index = 0usize;
            Q::for_each_entity(chunk, &cached.component_indices, &mut |item| {
                f(entities[entity_index], item);
                entity_index += 1;
            });
        }
    }
}

#[inline(always)]
pub(crate) fn run_for_each_chunk<Q, F>(world: &World, chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(Q::Chunk<'w>),
{
    for cached in chunks {
        unsafe {
            f(Q::chunk_from_raw(
                cached.chunk(world),
                &cached.component_indices,
            ));
        }
    }
}

#[inline(always)]
pub(crate) fn run_for_each_chunk_with_entities<Q, F>(
    world: &World,
    chunks: &[SequentialChunk],
    mut f: F,
) where
    Q: QuerySpec,
    F: for<'w> FnMut(&'w [EntityId], Q::Chunk<'w>),
{
    for cached in chunks {
        unsafe {
            let chunk = cached.chunk(world);
            f(
                chunk.entities(),
                Q::chunk_from_raw(chunk, &cached.component_indices),
            );
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CacheState {
    #[default]
    Observe,
    Ready,
    PreferDirect,
}

#[derive(Default)]
pub(crate) struct SequentialChunkCache {
    world: Option<Arc<()>>,
    chunk_set_epoch: Option<u64>,
    chunks: Vec<SequentialChunk>,
    state: CacheState,
    #[cfg(test)]
    rebuild_count: usize,
}

impl SequentialChunkCache {
    /// Returns a flattened non-empty chunk plan once the same chunk set has
    /// been observed twice in a row.
    ///
    /// The first call after a structural change stays on the direct archetype
    /// path. This avoids paying to construct a cache in workloads that mutate
    /// storage between every query execution, while stable prepared queries
    /// amortize the plan after one observation.
    pub(crate) fn get_or_prepare<'a>(
        &'a mut self,
        archetypes: &[CachedArchetype],
        world: &World,
    ) -> Option<&'a [SequentialChunk]> {
        let same_world = self
            .world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        let same_chunks = same_world && self.chunk_set_epoch == Some(world.chunk_set_epoch());

        if !same_chunks {
            self.world = Some(Arc::clone(world.cache_token()));
            self.chunk_set_epoch = Some(world.chunk_set_epoch());
            self.chunks.clear();
            self.state = CacheState::Observe;
            return None;
        }

        match self.state {
            CacheState::Ready => return Some(&self.chunks),
            CacheState::PreferDirect => return None,
            CacheState::Observe => {}
        }

        self.chunks.clear();
        self.chunks.reserve(
            archetypes
                .iter()
                .map(|cached| world.data[cached.data_index].chunks.len())
                .sum(),
        );
        let mut total_entities = 0usize;
        for cached in archetypes {
            let data = &world.data[cached.data_index];
            for (chunk_index, chunk) in data.chunks.iter().enumerate() {
                debug_assert!(chunk.entity_count != 0);
                let chunk_id = data.chunk_ids[chunk_index];
                debug_assert!(chunk_id.is_assigned());
                total_entities += chunk.entity_count;
                self.chunks.push(SequentialChunk {
                    chunk_id,
                    component_indices: cached.component_indices,
                });
            }
        }

        // Flattening removes archetype metadata traversal, but stable chunk
        // route resolution has its own cost. Keep the direct nested traversal
        // when work per chunk already amortizes that metadata, and flatten
        // only genuinely fragmented layouts.
        let fragmented = self.chunks.len() >= MIN_FLATTENED_CHUNKS
            && total_entities
                <= self
                    .chunks
                    .len()
                    .saturating_mul(MAX_AVERAGE_ENTITIES_PER_CHUNK);
        if !fragmented {
            self.chunks.clear();
            self.state = CacheState::PreferDirect;
            return None;
        }

        self.state = CacheState::Ready;
        #[cfg(test)]
        {
            self.rebuild_count += 1;
        }
        Some(&self.chunks)
    }

    /// Observes the current chunk set during a scheduler's serial
    /// preparation pass. Stable systems can then read the flattened plan
    /// without interior mutation while they execute.
    #[inline]
    pub(crate) fn prepare(&mut self, archetypes: &[CachedArchetype], world: &World) {
        let _ = self.get_or_prepare(archetypes, world);
    }

    /// Returns a previously prepared plan if it still belongs to this exact
    /// World chunk set.
    #[inline(always)]
    pub(crate) fn current<'a>(&'a self, world: &World) -> Option<&'a [SequentialChunk]> {
        let same_world = self
            .world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        (self.state == CacheState::Ready
            && same_world
            && self.chunk_set_epoch == Some(world.chunk_set_epoch()))
        .then_some(self.chunks.as_slice())
    }
}

#[cfg(test)]
impl SequentialChunkCache {
    pub(crate) fn rebuild_count(&self) -> usize {
        self.rebuild_count
    }
}

#[cfg(test)]
mod tests {
    use crate::ecs::{PreparedQuery, World};

    struct Position;
    struct MatchA;
    struct MatchB;
    struct MatchC;
    struct MatchD;
    struct MatchE;
    struct MatchF;
    struct MatchG;

    #[test]
    fn chunk_plan_survives_in_chunk_row_churn() {
        let mut world = World::new();
        world.spawn((Position,));
        world.spawn((Position, MatchA));
        world.spawn((Position, MatchB));
        world.spawn((Position, MatchC));
        world.spawn((Position, MatchD));
        world.spawn((Position, MatchE));
        world.spawn((Position, MatchF));
        world.spawn((Position, MatchG));

        let mut query = PreparedQuery::<&Position>::new();
        query.for_each(&world, |_| {});
        query.for_each(&world, |_| {});
        assert_eq!(query.sequential_rebuild_count(), 1);

        let added = world.spawn((Position,));
        for _ in 0..2 {
            let mut seen = 0;
            query.for_each(&world, |_| seen += 1);
            assert_eq!(seen, 9);
        }
        assert_eq!(query.sequential_rebuild_count(), 1);

        assert!(world.despawn(added));
        for _ in 0..2 {
            let mut seen = 0;
            query.for_each(&world, |_| seen += 1);
            assert_eq!(seen, 8);
        }
        assert_eq!(query.sequential_rebuild_count(), 1);
    }
}

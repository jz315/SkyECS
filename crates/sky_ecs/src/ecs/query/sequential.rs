use super::{CachedArchetype, Chunk, ComponentIndexMap, EntityId, QuerySpec, World};
use std::sync::Arc;

const MIN_FLATTENED_CHUNKS: usize = 8;
const MAX_AVERAGE_ENTITIES_PER_CHUNK: usize = 512;

#[derive(Clone, Copy)]
struct RawChunkPtr(*const Chunk);

// Safety: cached pointers are dereferenced only after validating both the
// originating World identity and its storage epoch. Any mutation that can move
// a Chunk increments the epoch before changing storage.
unsafe impl Send for RawChunkPtr {}
unsafe impl Sync for RawChunkPtr {}

#[derive(Clone, Copy)]
pub(crate) struct SequentialChunk {
    chunk: RawChunkPtr,
    pub(crate) component_indices: ComponentIndexMap,
}

impl SequentialChunk {
    #[inline(always)]
    pub(crate) unsafe fn chunk<'w>(&self) -> &'w Chunk {
        // Safety: `SequentialChunkCache::get_or_prepare` returns cached
        // pointers only after matching both the World identity and the
        // storage epoch. The caller also holds that World borrowed for `'w`.
        unsafe { &*self.chunk.0 }
    }
}

#[inline(always)]
pub(crate) fn run_for_each<Q, F>(chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(Q::Item<'w>),
{
    for cached in chunks {
        unsafe {
            Q::for_each_entity(cached.chunk(), &cached.component_indices, &mut f);
        }
    }
}

#[inline(always)]
pub(crate) fn run_for_each_with_entity<Q, F>(chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(EntityId, Q::Item<'w>),
{
    for cached in chunks {
        unsafe {
            let chunk = cached.chunk();
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
pub(crate) fn run_for_each_chunk<Q, F>(chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(Q::Chunk<'w>),
{
    for cached in chunks {
        unsafe {
            f(Q::chunk_from_raw(cached.chunk(), &cached.component_indices));
        }
    }
}

#[inline(always)]
pub(crate) fn run_for_each_chunk_with_entities<Q, F>(chunks: &[SequentialChunk], mut f: F)
where
    Q: QuerySpec,
    F: for<'w> FnMut(&'w [EntityId], Q::Chunk<'w>),
{
    for cached in chunks {
        unsafe {
            let chunk = cached.chunk();
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
    storage_epoch: Option<u64>,
    chunks: Vec<SequentialChunk>,
    state: CacheState,
    #[cfg(test)]
    rebuild_count: usize,
}

impl SequentialChunkCache {
    /// Returns a flattened non-empty chunk plan once the same storage layout
    /// has been observed twice in a row.
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
        let same_storage = same_world && self.storage_epoch == Some(world.storage_epoch());

        if !same_storage {
            self.world = Some(Arc::clone(world.cache_token()));
            self.storage_epoch = Some(world.storage_epoch());
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
            for chunk in &data.chunks {
                debug_assert!(chunk.entity_count != 0);
                total_entities += chunk.entity_count;
                self.chunks.push(SequentialChunk {
                    chunk: RawChunkPtr(chunk),
                    component_indices: cached.component_indices,
                });
            }
        }

        // Flattening removes metadata pointer chasing, but indirect raw chunk
        // access can inhibit code generation for long dense loops. Keep the
        // direct nested traversal when work per chunk already amortizes its
        // metadata cost, and flatten only genuinely fragmented layouts.
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

    /// Observes the current storage layout during a scheduler's serial
    /// preparation pass. Stable systems can then read the flattened plan
    /// without interior mutation while they execute.
    #[inline]
    pub(crate) fn prepare(&mut self, archetypes: &[CachedArchetype], world: &World) {
        let _ = self.get_or_prepare(archetypes, world);
    }

    /// Returns a previously prepared plan if it still belongs to this exact
    /// World storage layout.
    #[inline(always)]
    pub(crate) fn current<'a>(&'a self, world: &World) -> Option<&'a [SequentialChunk]> {
        let same_world = self
            .world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        (self.state == CacheState::Ready
            && same_world
            && self.storage_epoch == Some(world.storage_epoch()))
        .then_some(self.chunks.as_slice())
    }
}

#[cfg(test)]
impl SequentialChunkCache {
    pub(crate) fn rebuild_count(&self) -> usize {
        self.rebuild_count
    }
}

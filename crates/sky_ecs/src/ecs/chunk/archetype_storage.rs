use super::{
    Archetype, Chunk, ChunkLayout, EntityId, CHUNK_SIZE_TIERS, CHUNK_TIER_COUNT, MAX_CHUNK_SIZE,
    REPEATED_TIER_START, SMALL_CHUNK_SIZE, TINY_CHUNK_SIZE,
};
use smallvec::SmallVec;

pub(crate) struct ArchetypeStorage {
    pub archetype: Archetype,
    pub(super) layouts: SmallVec<[ChunkLayout; CHUNK_TIER_COUNT]>,
    pub chunks: Vec<Chunk>,
}

#[derive(Clone, Copy)]
enum GrowthAction {
    Promote(usize),
    Append(usize),
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkEntityLocation {
    pub chunk_index: usize,
    pub entity_index: usize,
}

impl ArchetypeStorage {
    pub fn new(archetype: Archetype) -> Self {
        let component_bytes = archetype
            .components
            .iter()
            .try_fold(0usize, |bytes, component| bytes.checked_add(component.size))
            .expect("archetype component sizes overflow usize");
        let mut layouts: SmallVec<[ChunkLayout; CHUNK_TIER_COUNT]> = SmallVec::new();
        for chunk_size in CHUNK_SIZE_TIERS {
            let layout = if component_bytes == 0 {
                Some(ChunkLayout {
                    chunk_size: chunk_size as u32,
                    max_entity_count: chunk_size as u32,
                })
            } else {
                ChunkLayout::try_for_archetype_with_component_bytes(
                    archetype,
                    chunk_size,
                    component_bytes,
                )
            };
            if let Some(layout) = layout {
                layouts.push(layout);
            }
        }
        if layouts
            .last()
            .is_none_or(|layout| layout.chunk_size() != MAX_CHUNK_SIZE)
        {
            layouts.clear();
            layouts.push(ChunkLayout::exact_one_entity(archetype).unwrap_or_else(|| {
                panic!("archetype is too large to represent with 32-bit chunk offsets")
            }));
        }

        Self {
            archetype,
            layouts,
            chunks: Vec::new(),
        }
    }

    pub(super) fn layout_index_after(&self, chunk_size: usize) -> usize {
        self.layouts
            .iter()
            .position(|layout| layout.chunk_size() > chunk_size)
            .unwrap_or(self.layouts.len() - 1)
    }

    pub(super) fn add_chunk_with_layout(&mut self, layout_index: usize) {
        if self.chunks.capacity() == 0 {
            // Incremental worlds commonly leave most archetypes with a single
            // chunk. Avoid Vec's default four-Chunk first allocation; known
            // batches reserve their complete chunk count before reaching here.
            self.chunks.reserve_exact(1);
        }
        let layout = &self.layouts[layout_index];
        let chunk = Chunk::new_with_layout(self.archetype, layout);
        self.chunks.push(chunk);
    }

    fn next_growth_action(
        &self,
        current_layout_index: usize,
        incoming_entities: usize,
        same_size_tail_count: usize,
    ) -> GrowthAction {
        let current_size = self.layouts[current_layout_index].chunk_size();
        let next_layout_index = self.layout_index_after(current_size);

        if current_size == TINY_CHUNK_SIZE {
            return GrowthAction::Promote(next_layout_index);
        }

        let append_layout_index = if current_size < REPEATED_TIER_START {
            self.layout_index_for_capacity(incoming_entities)
                .max(next_layout_index)
        } else if same_size_tail_count < 4 {
            current_layout_index
        } else {
            next_layout_index
        };
        GrowthAction::Append(append_layout_index)
    }

    /// Predicts how many chunks an empty storage will create for a guaranteed
    /// batch. This mirrors `grow_tail` without allocating component storage.
    pub(super) fn batch_chunk_count(&self, entity_count: usize) -> usize {
        let mut remaining = entity_count.max(1);
        let mut layout_index = self.layout_index_for_batch(remaining);
        let mut current_capacity = self.layouts[layout_index].max_entity_count();
        let mut chunk_count = 1usize;
        let mut same_size_tail_count = 1usize;

        loop {
            if remaining <= current_capacity {
                return chunk_count;
            }
            remaining -= current_capacity;

            match self.next_growth_action(layout_index, remaining, same_size_tail_count) {
                GrowthAction::Promote(next_layout_index) => {
                    let promoted_capacity = self.layouts[next_layout_index].max_entity_count();
                    debug_assert!(promoted_capacity >= current_capacity);
                    let added_capacity = promoted_capacity - current_capacity;
                    if remaining <= added_capacity {
                        return chunk_count;
                    }
                    remaining -= added_capacity;
                    layout_index = next_layout_index;
                    current_capacity = promoted_capacity;
                    same_size_tail_count = 1;
                }
                GrowthAction::Append(append_layout_index) => {
                    if append_layout_index == layout_index {
                        same_size_tail_count += 1;
                    } else {
                        same_size_tail_count = 1;
                    }
                    layout_index = append_layout_index;
                    current_capacity = self.layouts[layout_index].max_entity_count();
                    chunk_count += 1;
                }
            }
        }
    }

    fn additional_chunk_count(&self, incoming_entities: usize) -> usize {
        let Some(tail) = self.chunks.last() else {
            return self.batch_chunk_count(incoming_entities);
        };
        let available = tail.max_entity_count - tail.entity_count;
        let mut remaining = incoming_entities.saturating_sub(available);
        if remaining == 0 {
            return 0;
        }

        let mut layout_index = self
            .layouts
            .iter()
            .position(|layout| layout.chunk_size() == tail.block_size())
            .expect("active chunk must use a registered size class");
        let mut current_capacity = self.layouts[layout_index].max_entity_count();
        let mut same_size_tail_count = self
            .chunks
            .iter()
            .rev()
            .take(4)
            .take_while(|chunk| chunk.block_size() == tail.block_size())
            .count();
        let mut additional_chunks = 0usize;

        loop {
            match self.next_growth_action(layout_index, remaining, same_size_tail_count) {
                GrowthAction::Promote(next_layout_index) => {
                    let promoted_capacity = self.layouts[next_layout_index].max_entity_count();
                    let added_capacity = promoted_capacity - current_capacity;
                    if remaining <= added_capacity {
                        return additional_chunks;
                    }
                    remaining -= added_capacity;
                    layout_index = next_layout_index;
                    current_capacity = promoted_capacity;
                    same_size_tail_count = 1;
                }
                GrowthAction::Append(next_layout_index) => {
                    additional_chunks += 1;
                    let capacity = self.layouts[next_layout_index].max_entity_count();
                    if remaining <= capacity {
                        return additional_chunks;
                    }
                    remaining -= capacity;
                    if next_layout_index == layout_index {
                        same_size_tail_count += 1;
                    } else {
                        same_size_tail_count = 1;
                    }
                    layout_index = next_layout_index;
                    current_capacity = capacity;
                }
            }
        }
    }

    fn layout_index_for_capacity(&self, entity_count: usize) -> usize {
        self.layouts
            .iter()
            .position(|layout| layout.max_entity_count() >= entity_count)
            .unwrap_or(self.layouts.len() - 1)
    }

    fn layout_index_for_batch(&self, entity_count: usize) -> usize {
        self.layouts
            .iter()
            .position(|layout| layout.max_entity_count().saturating_mul(4) >= entity_count)
            .unwrap_or(self.layouts.len() - 1)
    }

    pub(super) fn grow_tail(&mut self, incoming_entities: usize) {
        let Some(current_size) = self.chunks.last().map(Chunk::block_size) else {
            let layout_index = self.layout_index_for_capacity(incoming_entities);
            self.add_chunk_with_layout(layout_index);
            return;
        };

        let current_layout_index = self
            .layouts
            .iter()
            .position(|layout| layout.chunk_size() == current_size)
            .expect("active chunk must use a registered size class");
        let same_size_tail_count = self
            .chunks
            .iter()
            .rev()
            .take(4)
            .take_while(|chunk| chunk.block_size() == current_size)
            .count();
        match self.next_growth_action(
            current_layout_index,
            incoming_entities,
            same_size_tail_count,
        ) {
            GrowthAction::Promote(next_layout_index) => {
                let layout = self.layouts[next_layout_index];
                debug_assert_eq!(layout.chunk_size(), SMALL_CHUNK_SIZE);
                self.chunks.last_mut().unwrap().promote_tiny(&layout);
            }
            GrowthAction::Append(layout_index) => self.add_chunk_with_layout(layout_index),
        }
    }

    /// Returns a tail chunk with at least one available row, growing the
    /// storage once when the current tail is full.
    #[inline]
    pub(crate) fn ensure_batch_tail(&mut self, guaranteed_remaining: usize) -> usize {
        if self.chunks.last().is_none_or(Chunk::is_full) {
            self.grow_tail(guaranteed_remaining.max(1));
        }

        let chunk_index = self.chunks.len() - 1;
        debug_assert!(!self.chunks[chunk_index].is_full());
        chunk_index
    }

    /// Uses a guaranteed batch size to choose the first block before the hot
    /// insertion loop starts. Existing chunks are never relocated; a partial
    /// tail is filled before normal incremental growth appends another block.
    pub(crate) fn prepare_batch_capacity(&mut self, guaranteed_entities: usize) {
        let guaranteed_entities = guaranteed_entities.max(1);
        let Some(tail) = self.chunks.last() else {
            let layout_index = self.layout_index_for_batch(guaranteed_entities);
            let chunk_count = self.batch_chunk_count(guaranteed_entities);
            self.chunks.reserve_exact(chunk_count);
            self.add_chunk_with_layout(layout_index);
            return;
        };

        let available = tail.max_entity_count - tail.entity_count;
        let tail_size = tail.block_size();
        let additional_chunks = self.additional_chunk_count(guaranteed_entities);
        if additional_chunks > 0 {
            self.chunks.reserve_exact(additional_chunks);
        }
        if available < guaranteed_entities && tail_size == TINY_CHUNK_SIZE {
            let layout_index = self.layout_index_after(TINY_CHUNK_SIZE);
            let layout = self.layouts[layout_index];
            debug_assert_eq!(layout.chunk_size(), SMALL_CHUNK_SIZE);
            self.chunks.last_mut().unwrap().promote_tiny(&layout);
        }
    }

    #[inline(always)]
    /// Adds a logical entity slot without initializing component columns.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component column at the returned
    /// location before the entity is observed or any destructor can run.
    pub(crate) unsafe fn add_entity(&mut self, entity: EntityId) -> ChunkEntityLocation {
        unsafe { self.add_entity_with_batch_hint(entity, 1) }
    }

    /// Adds an entity while using a guaranteed remaining batch size when the
    /// current tail needs to grow.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component column at the returned
    /// location before the entity is observed or any destructor can run.
    #[inline(always)]
    pub(crate) unsafe fn add_entity_with_batch_hint(
        &mut self,
        entity: EntityId,
        guaranteed_remaining: usize,
    ) -> ChunkEntityLocation {
        if let Some(chunk) = self.chunks.last_mut() {
            if let Some(entity_index) = unsafe { chunk.add_entity(entity) } {
                return ChunkEntityLocation {
                    chunk_index: self.chunks.len() - 1,
                    entity_index,
                };
            }
        }

        self.grow_tail(guaranteed_remaining.max(1));
        let chunk = self.chunks.last_mut().unwrap();
        let entity_index = unsafe { chunk.add_entity(entity) }.unwrap();
        ChunkEntityLocation {
            chunk_index: self.chunks.len() - 1,
            entity_index,
        }
    }

    #[inline(always)]
    pub fn remove_entity(
        &mut self,
        location: ChunkEntityLocation,
    ) -> Option<(EntityId, ChunkEntityLocation)> {
        if self.chunks.is_empty() {
            return None;
        }

        let last_chunk_index = self.chunks.len() - 1;
        let last_entity_index = self.chunks[last_chunk_index].entity_count.checked_sub(1)?;

        let removed_is_last =
            location.chunk_index == last_chunk_index && location.entity_index == last_entity_index;

        let moved_entity = if removed_is_last {
            None
        } else {
            let moved_entity = self.chunks[last_chunk_index]
                .entity_id(last_entity_index)
                .unwrap();

            if location.chunk_index == last_chunk_index {
                self.chunks[last_chunk_index]
                    .copy_entity_within(last_entity_index, location.entity_index);
            } else {
                let (head, tail) = self.chunks.split_at_mut(last_chunk_index);
                let dst_chunk = &mut head[location.chunk_index];
                let src_chunk = &tail[0];
                dst_chunk.copy_entity_from(src_chunk, last_entity_index, location.entity_index);
            }

            Some((
                moved_entity,
                ChunkEntityLocation {
                    chunk_index: location.chunk_index,
                    entity_index: location.entity_index,
                },
            ))
        };

        self.chunks[last_chunk_index].remove_last_entity();
        if self.chunks.last().is_some_and(Chunk::is_empty) {
            // Dropping the empty chunk returns its block to the bounded,
            // thread-local pool shared by every archetype on this thread.
            self.chunks.pop();
        }

        moved_entity
    }
}

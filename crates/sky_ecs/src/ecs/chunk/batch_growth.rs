use super::{
    batch_plan::BatchGrowthPlan, ArchetypeStorage, Chunk, SMALL_CHUNK_SIZE, TINY_CHUNK_SIZE,
};

impl ArchetypeStorage {
    pub(super) fn grow_batch_tail(
        &mut self,
        plan: &mut BatchGrowthPlan,
        guaranteed_remaining: usize,
    ) {
        if let Some(layout) = plan.take_next_layout() {
            self.add_chunk(layout);
        } else {
            self.grow_tail(guaranteed_remaining.max(1));
        }
    }

    #[inline]
    pub(crate) fn ensure_planned_batch_tail(
        &mut self,
        plan: &mut BatchGrowthPlan,
        guaranteed_remaining: usize,
    ) -> usize {
        if self.chunks.last().is_none_or(Chunk::is_full) {
            self.grow_batch_tail(plan, guaranteed_remaining);
        }

        let chunk_index = self.chunks.len() - 1;
        debug_assert!(!self.chunks[chunk_index].is_full());
        chunk_index
    }

    /// Reserves vector slots and returns the operation-owned batch plan.
    /// Physical chunks from the plan are consumed largest first.
    pub(crate) fn prepare_batch_capacity(&mut self, guaranteed_entities: usize) -> BatchGrowthPlan {
        let guaranteed_entities = guaranteed_entities.max(1);
        let Some(tail_size) = self.chunks.last().map(Chunk::block_size) else {
            let mut plan = BatchGrowthPlan::for_remaining(
                self.archetype,
                guaranteed_entities,
                TINY_CHUNK_SIZE,
            );
            let planned_chunks = plan.remaining_chunk_count();
            if planned_chunks > 0 {
                self.chunks.reserve_exact(planned_chunks);
                self.chunk_ids.reserve_exact(planned_chunks);
                self.add_chunk(plan.take_next_layout().unwrap());
                return plan;
            }

            let layout_index = self.layout_index_for_batch(guaranteed_entities);
            let chunk_count = self.batch_chunk_count(guaranteed_entities);
            self.chunks.reserve_exact(chunk_count);
            self.chunk_ids.reserve_exact(chunk_count);
            self.add_chunk_with_layout(layout_index);
            return plan;
        };

        if self.tail_available() < guaranteed_entities && tail_size == TINY_CHUNK_SIZE {
            let layout_index = self.layout_index_after(TINY_CHUNK_SIZE);
            let layout = self.layouts[layout_index];
            debug_assert_eq!(layout.chunk_size(), SMALL_CHUNK_SIZE);
            self.mark_column_bases_changed();
            self.chunks.last_mut().unwrap().promote_tiny(&layout);
        }

        let remaining = guaranteed_entities.saturating_sub(self.tail_available());
        let minimum_chunk_size = self.next_batch_chunk_size(remaining.max(1));
        let plan = BatchGrowthPlan::for_remaining(self.archetype, remaining, minimum_chunk_size);
        let additional_chunks = if plan.remaining_chunk_count() > 0 {
            plan.remaining_chunk_count()
        } else {
            self.additional_chunk_count(guaranteed_entities)
        };
        if additional_chunks > 0 {
            self.chunks.reserve_exact(additional_chunks);
            self.chunk_ids.reserve_exact(additional_chunks);
        }
        plan
    }

    #[inline]
    fn tail_available(&self) -> usize {
        self.chunks
            .last()
            .map(|tail| tail.max_entity_count - tail.entity_count)
            .unwrap_or(0)
    }
}

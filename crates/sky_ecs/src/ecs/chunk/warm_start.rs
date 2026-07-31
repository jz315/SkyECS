use super::{ArchetypeStorage, TINY_CHUNK_SIZE};

/// Empty storages remember at most this established growth tier.
///
/// The physical chunk is still destroyed and returned to the existing
/// bounded TLS pool. This hint only prevents a repeatedly active archetype
/// from replaying the tiny-to-large growth ladder on every activation.
const WARM_START_MAX_CHUNK_SIZE: usize = 64 * 1024;

impl ArchetypeStorage {
    #[inline(always)]
    pub(super) fn warm_start_layout_index(&self) -> usize {
        self.warm_start_layout_index as usize
    }

    #[inline(always)]
    pub(super) fn warm_start_minimum_chunk_size(&self) -> usize {
        self.layouts[self.warm_start_layout_index()].chunk_size()
    }

    #[inline]
    pub(super) fn warm_start_layout_for_capacity(&self, entity_count: usize) -> usize {
        let required = self
            .layouts
            .iter()
            .position(|layout| layout.max_entity_count() >= entity_count)
            .unwrap_or(self.layouts.len() - 1);
        required.max(self.warm_start_layout_index())
    }

    #[inline]
    pub(super) fn observe_retired_chunk_size(&mut self, chunk_size: usize) {
        let bounded_size = chunk_size.min(WARM_START_MAX_CHUNK_SIZE);
        let Some(layout_index) = self
            .layouts
            .iter()
            .rposition(|layout| layout.chunk_size() <= bounded_size)
        else {
            return;
        };
        self.warm_start_layout_index = self.warm_start_layout_index.max(
            layout_index
                .try_into()
                .expect("chunk layout tier limit exceeded"),
        );
    }

    #[cfg(test)]
    pub(super) fn warm_start_chunk_size(&self) -> usize {
        self.warm_start_minimum_chunk_size()
    }
}

const _: () = assert!(TINY_CHUNK_SIZE <= WARM_START_MAX_CHUNK_SIZE);

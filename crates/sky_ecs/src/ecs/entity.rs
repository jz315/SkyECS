/// A lightweight, generational handle to an entity in a [`World`](super::World).
///
/// Entity IDs are composed of a **slot index** (which may be reused) and a
/// **generation counter** that increments each time the slot is recycled.
/// This means stale IDs are automatically detected — calling
/// [`World::get`](super::World::get) with a despawned entity's ID returns
/// `None` rather than accessing a different entity that reused the slot.
///
/// `EntityId` is `Copy`, `Eq`, and `Hash`, so it can be used freely as a
/// key in collections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    index: u32,
    generation: u32,
}

impl EntityId {
    /// Creates an entity id from its raw slot index and generation.
    ///
    /// This is mainly for low-level tests and renderer sort keys. IDs created
    /// this way are not guaranteed to refer to a live entity in any
    /// [`World`](super::World).
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the raw slot index.
    ///
    /// This is an internal detail and should not be relied upon for identity
    /// comparisons — use `PartialEq` instead.
    #[inline(always)]
    pub fn index(self) -> u32 {
        self.index
    }

    /// Returns the generation counter for this entity.
    ///
    /// The generation increments each time the slot is reused after a
    /// despawn, allowing stale handles to be detected.
    #[inline(always)]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

use super::ChunkId;

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityLocation {
    pub data_index: usize,
    pub chunk_index: usize,
    pub entity_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityRoute {
    pub chunk_id: ChunkId,
    pub entity_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityRecord {
    // Three u32 values keep the random-access working set at 12 bytes per
    // entity. An unassigned chunk route represents a vacant entity slot.
    pub generation: u32,
    chunk_id: ChunkId,
    entity_index: u32,
}

impl EntityRecord {
    #[inline(always)]
    pub(crate) fn vacant(generation: u32) -> Self {
        Self {
            generation,
            chunk_id: ChunkId::UNASSIGNED,
            entity_index: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn occupied_indices(generation: u32, chunk_id: ChunkId, entity_index: u32) -> Self {
        debug_assert!(chunk_id.is_assigned());
        Self {
            generation,
            chunk_id,
            entity_index,
        }
    }

    /// Appends a record after its vector capacity has already been reserved.
    ///
    /// # Safety
    ///
    /// `records` must have capacity for at least one additional element.
    #[inline(always)]
    pub(crate) unsafe fn append_reserved(records: &mut Vec<Self>, record: Self) {
        debug_assert!(records.len() < records.capacity());
        let len = records.len();
        unsafe {
            records.as_mut_ptr().add(len).write(record);
            records.set_len(len + 1);
        }
    }

    #[inline(always)]
    pub(crate) fn occupied(generation: u32, route: EntityRoute) -> Self {
        let mut record = Self::vacant(generation);
        record.set_route(route);
        record
    }

    #[inline(always)]
    pub(crate) fn route(self) -> Option<EntityRoute> {
        if !self.chunk_id.is_assigned() {
            return None;
        }

        Some(EntityRoute {
            chunk_id: self.chunk_id,
            entity_index: self.entity_index as usize,
        })
    }

    #[inline(always)]
    pub(crate) fn set_route(&mut self, route: EntityRoute) {
        debug_assert!(route.chunk_id.is_assigned());
        self.chunk_id = route.chunk_id;
        self.entity_index =
            u32::try_from(route.entity_index).expect("chunk entity index limit exhausted");
    }

    #[inline(always)]
    pub(crate) fn set_route_indices(&mut self, chunk_id: ChunkId, entity_index: u32) {
        debug_assert!(chunk_id.is_assigned());
        self.chunk_id = chunk_id;
        self.entity_index = entity_index;
    }

    #[inline(always)]
    pub(crate) fn clear_route(&mut self) {
        self.chunk_id = ChunkId::UNASSIGNED;
    }

    #[inline(always)]
    pub(crate) fn is_alive(self) -> bool {
        self.chunk_id.is_assigned()
    }
}

#[cfg(test)]
mod tests {
    use super::{ChunkId, EntityRecord, EntityRoute};

    #[test]
    fn entity_record_stays_cache_dense() {
        assert_eq!(std::mem::size_of::<EntityRecord>(), 12);

        let mut record = EntityRecord::vacant(7);
        assert!(!record.is_alive());
        assert!(record.route().is_none());

        let route = EntityRoute {
            chunk_id: ChunkId(5),
            entity_index: 8,
        };
        record.set_route(route);
        assert!(record.is_alive());
        let actual = record.route().unwrap();
        assert_eq!(actual.chunk_id, route.chunk_id);
        assert_eq!(actual.entity_index, route.entity_index);

        record.clear_route();
        assert!(!record.is_alive());
        assert!(record.route().is_none());
    }
}

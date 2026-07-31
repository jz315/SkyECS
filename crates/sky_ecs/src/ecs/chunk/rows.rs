use super::{ArchetypeStorage, Chunk, EntityId};
use crate::ecs::entity::EntityRoute;
use crate::ecs::ChunkId;

#[derive(Clone, Copy, Debug)]
pub struct ChunkEntityLocation {
    pub chunk_index: usize,
    pub entity_index: usize,
}

pub(crate) struct ChunkRemoval {
    pub(crate) moved: Option<(EntityId, EntityRoute)>,
    pub(crate) retired_chunk: Option<ChunkId>,
}

impl Chunk {
    /// Appends a contiguous generation-zero EntityId range to rows whose
    /// component columns have already been initialized.
    ///
    /// # Safety
    ///
    /// The chunk must have `count` reserved entity slots and logical component
    /// capacity. `first_entity_id + count` must fit in `u32`. No row in the
    /// appended range may become observable until this function completes.
    #[inline]
    pub(crate) unsafe fn append_fresh_entity_ids(&mut self, first_entity_id: u32, count: usize) {
        debug_assert_eq!(self.entities.len(), self.entity_count);
        debug_assert!(self.entities.capacity() - self.entities.len() >= count);
        debug_assert!(self.max_entity_count - self.entity_count >= count);

        let entity_start = self.entity_count;
        let output = unsafe { self.entities.as_mut_ptr().add(entity_start) };
        for offset in 0..count {
            unsafe {
                output
                    .add(offset)
                    .write(EntityId::new(first_entity_id + offset as u32, 0));
            }
        }
        unsafe {
            self.entities.set_len(entity_start + count);
        }
        self.entity_count = entity_start + count;
    }
}

impl ArchetypeStorage {
    /// Attempts to append one row without changing the physical chunk set or
    /// relocating component columns.
    ///
    /// A `None` result means the caller must enter the guarded growth path.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component in the returned row before
    /// the storage can be observed, migrated, removed, or dropped.
    #[inline(always)]
    pub(crate) unsafe fn try_add_entity_to_tail(
        &mut self,
        entity: EntityId,
    ) -> Option<ChunkEntityLocation> {
        let chunk_index = self.chunks.len().checked_sub(1)?;
        let entity_index = unsafe {
            self.chunks
                .get_unchecked_mut(chunk_index)
                .add_entity(entity)?
        };
        Some(ChunkEntityLocation {
            chunk_index,
            entity_index,
        })
    }

    /// Adds one row, growing or promoting the tail when the steady-state path
    /// has no capacity.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component in the returned row before
    /// the storage can be observed, migrated, removed, or dropped.
    #[inline(always)]
    pub(crate) unsafe fn add_entity(&mut self, entity: EntityId) -> ChunkEntityLocation {
        unsafe { self.add_entity_with_batch_hint(entity, 1) }
    }

    /// Adds an entity while using a guaranteed remaining batch size when the
    /// current tail needs to grow.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component column for the returned row
    /// before the storage can be observed, migrated, removed, or dropped.
    #[inline(always)]
    pub(crate) unsafe fn add_entity_with_batch_hint(
        &mut self,
        entity: EntityId,
        guaranteed_remaining: usize,
    ) -> ChunkEntityLocation {
        if let Some(location) = unsafe { self.try_add_entity_to_tail(entity) } {
            return location;
        }

        self.grow_tail(guaranteed_remaining.max(1));
        let chunk_index = self.chunks.len() - 1;
        let chunk = self.chunks.last_mut().unwrap();
        let entity_index = unsafe { chunk.add_entity(entity) }.unwrap();
        ChunkEntityLocation {
            chunk_index,
            entity_index,
        }
    }

    /// Returns whether removing any live row will empty and retire the current
    /// tail chunk under the active storage policy.
    #[inline(always)]
    pub(crate) fn next_removal_retires_tail(&self) -> bool {
        self.chunks
            .last()
            .is_some_and(|chunk| chunk.entity_count == 1)
    }

    #[inline(always)]
    fn move_tail_entity_into(
        &mut self,
        location: ChunkEntityLocation,
        last_chunk_index: usize,
        last_entity_index: usize,
    ) -> Option<(EntityId, EntityRoute)> {
        let removed_is_last =
            location.chunk_index == last_chunk_index && location.entity_index == last_entity_index;
        if removed_is_last {
            return None;
        }

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

        let chunk_id = self.chunk_ids[location.chunk_index];
        debug_assert!(chunk_id.is_assigned());
        Some((
            moved_entity,
            EntityRoute {
                chunk_id,
                entity_index: location.entity_index,
            },
        ))
    }

    /// Swap-removes a row while the tail is known to remain live.
    #[inline(always)]
    pub(crate) fn remove_entity_stable(&mut self, location: ChunkEntityLocation) -> ChunkRemoval {
        let last_chunk_index = self.chunks.len() - 1;
        debug_assert!(self.chunks[last_chunk_index].entity_count > 1);
        let last_entity_index = self.chunks[last_chunk_index].entity_count - 1;
        let moved = self.move_tail_entity_into(location, last_chunk_index, last_entity_index);
        self.chunks[last_chunk_index].remove_last_entity();
        ChunkRemoval {
            moved,
            retired_chunk: None,
        }
    }

    #[inline(always)]
    pub fn remove_entity(&mut self, location: ChunkEntityLocation) -> ChunkRemoval {
        let Some(last_chunk_index) = self.chunks.len().checked_sub(1) else {
            return ChunkRemoval {
                moved: None,
                retired_chunk: None,
            };
        };
        let Some(last_entity_index) = self.chunks[last_chunk_index].entity_count.checked_sub(1)
        else {
            return ChunkRemoval {
                moved: None,
                retired_chunk: None,
            };
        };
        if last_entity_index > 0 {
            return self.remove_entity_stable(location);
        }

        let moved = self.move_tail_entity_into(location, last_chunk_index, last_entity_index);
        self.chunks[last_chunk_index].remove_last_entity();

        // A one-row tail always retires after the swap. Dropping it returns
        // both allocations to the existing bounded thread-local pool.
        let retired_size = self.chunks[last_chunk_index].block_size();
        self.observe_retired_chunk_size(retired_size);
        self.mark_chunk_set_changed();
        self.mark_column_bases_changed();
        self.chunks.pop();
        let retired_chunk = self
            .chunk_ids
            .pop()
            .filter(|chunk_id| chunk_id.is_assigned());
        debug_assert_eq!(self.chunks.len(), self.chunk_ids.len());

        ChunkRemoval {
            moved,
            retired_chunk,
        }
    }
}

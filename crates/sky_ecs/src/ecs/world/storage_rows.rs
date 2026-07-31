use super::*;

/// One newly allocated row together with the already-registered stable route
/// needed by its entity record.
#[derive(Clone, Copy)]
pub(super) struct AllocatedStorageRow {
    pub(super) chunk_index: usize,
    pub(super) entity_index: usize,
    chunk_id: ChunkId,
}

impl AllocatedStorageRow {
    #[inline(always)]
    pub(super) fn route(self) -> EntityRoute {
        EntityRoute {
            chunk_id: self.chunk_id,
            entity_index: self.entity_index,
        }
    }
}

impl World {
    #[inline(always)]
    fn ensure_chunk_route(&mut self, data_index: usize, chunk_index: usize) -> ChunkId {
        let directory = &mut self.chunk_directory;
        let storage = &mut self.data[data_index];
        directory.ensure(&mut storage.chunk_ids[chunk_index], data_index, chunk_index)
    }

    /// Allocates one uninitialized storage row and registers its physical
    /// chunk before an entity record can make that row observable.
    ///
    /// Rows appended to an existing tail cannot change component bases, the
    /// physical chunk set, or storage activity. They therefore bypass the
    /// topology epoch guard. Growth and promotion remain guarded so unwinding
    /// cannot leave route or pointer caches looking current.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component in the returned row before
    /// exposing the entity or allowing that row to be dropped.
    #[inline(always)]
    pub(super) unsafe fn allocate_storage_row(
        &mut self,
        data_index: usize,
        entity: EntityId,
    ) -> AllocatedStorageRow {
        if let Some(location) = unsafe { self.data[data_index].try_add_entity_to_tail(entity) } {
            let chunk_id = self.data[data_index].chunk_id(location.chunk_index);
            let chunk_id = if chunk_id.is_assigned() {
                chunk_id
            } else {
                self.ensure_chunk_route(data_index, location.chunk_index)
            };
            return AllocatedStorageRow {
                chunk_index: location.chunk_index,
                entity_index: location.entity_index,
                chunk_id,
            };
        }

        let location = {
            let mut storage =
                ChunkSetEpochGuard::new(&mut self.data[data_index], &mut self.storage_epochs);
            unsafe { storage.storage_mut().add_entity(entity) }
        };
        let chunk_id = self.ensure_chunk_route(data_index, location.chunk_index);
        AllocatedStorageRow {
            chunk_index: location.chunk_index,
            entity_index: location.entity_index,
            chunk_id,
        }
    }

    /// Removes one row. Only the operation that retires the tail chunk needs
    /// the topology guard; ordinary swap-remove changes rows but leaves every
    /// cached component base intact.
    #[inline(always)]
    pub(super) fn remove_storage_row(
        &mut self,
        data_index: usize,
        location: ChunkEntityLocation,
    ) -> ChunkRemoval {
        if self.data[data_index].next_removal_retires_tail() {
            let mut storage =
                ChunkSetEpochGuard::new(&mut self.data[data_index], &mut self.storage_epochs);
            storage.storage_mut().remove_entity(location)
        } else {
            self.data[data_index].remove_entity_stable(location)
        }
    }

    #[inline(always)]
    pub(super) fn finish_chunk_removal(&mut self, removal: ChunkRemoval) {
        match removal.moved {
            Some(MovedEntityRepair::Row {
                entity,
                entity_index,
            }) => {
                let record = &mut self.entities[entity.index() as usize];
                debug_assert_eq!(record.generation, entity.generation());
                record.set_entity_index(entity_index);
            }
            Some(MovedEntityRepair::Route { entity, route }) => {
                self.set_entity_route(entity, route);
            }
            None => {}
        }
        if let Some(retired_chunk) = removal.retired_chunk {
            self.chunk_directory.release(retired_chunk);
        }
    }

    #[inline(always)]
    pub(crate) fn entity_location(&self, entity: EntityId) -> Option<EntityLocation> {
        let route = self.entity_route(entity)?;
        let address = self
            .chunk_directory
            .resolve(route.chunk_id)
            .expect("live entity must reference a registered chunk");
        Some(EntityLocation {
            data_index: address.data_index,
            chunk_index: address.chunk_index,
            entity_index: route.entity_index,
        })
    }

    #[inline(always)]
    pub(crate) fn entity_route(&self, entity: EntityId) -> Option<EntityRoute> {
        EntityRecord::resolve(&self.entities, entity)
    }

    #[inline(always)]
    pub(crate) fn entity_records(&self) -> &[EntityRecord] {
        &self.entities
    }

    #[inline(always)]
    pub(super) fn set_entity_route(&mut self, entity: EntityId, route: EntityRoute) {
        debug_assert!(route.chunk_id.is_assigned());
        let record = &mut self.entities[entity.index() as usize];
        debug_assert_eq!(record.generation, entity.generation());
        record.set_route(route);
    }

    #[inline(always)]
    pub(crate) fn chunk_route_slot_count(&self) -> usize {
        self.chunk_directory.slot_count()
    }
}

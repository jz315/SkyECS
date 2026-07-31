use super::*;

impl World {
    /// Removes the physical row and recycles the entity record after any
    /// required component destruction has completed.
    #[inline(always)]
    fn finish_despawn(&mut self, entity: EntityId, location: EntityLocation) {
        let removal = self.remove_storage_row(
            location.data_index,
            ChunkEntityLocation {
                chunk_index: location.chunk_index,
                entity_index: location.entity_index,
            },
        );

        let record = &mut self.entities[entity.index() as usize];
        record.clear_route();
        if let Some(next_generation) = record.generation.checked_add(1) {
            record.generation = next_generation;
            self.free_entities.push(entity.index());
        }
        self.live_entity_count -= 1;
        self.finish_chunk_removal(removal);
    }

    /// Destroys an entity and drops all its components.
    ///
    /// Returns `true` if the entity existed and was removed,
    /// or `false` if the entity ID was stale or invalid.
    /// If a component destructor panics, removal and location repair finish
    /// before that panic resumes.
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        let Some(location) = self.entity_location(entity) else {
            return false;
        };

        self.bump_row_layout_epoch();

        if self.data[location.data_index]
            .archetype
            .drop_component_indices
            .is_empty()
        {
            self.finish_despawn(entity, location);
            return true;
        }

        let mut drop_panic = None;
        {
            let chunk = &self.data[location.data_index].chunks[location.chunk_index];
            // SAFETY: `location` identifies a live row. Every droppable
            // component is consumed exactly once here, and the row is removed
            // before a captured panic is resumed.
            unsafe {
                Self::drop_entity_components_catching(
                    chunk,
                    location.entity_index,
                    &mut drop_panic,
                );
            }
        }

        self.finish_despawn(entity, location);

        if let Some(payload) = drop_panic {
            std::panic::resume_unwind(payload);
        }
        true
    }
}

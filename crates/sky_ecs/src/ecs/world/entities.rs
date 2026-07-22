use super::*;

struct BatchCommitGuard<'a> {
    live_entity_count: &'a mut usize,
    inserted: usize,
}

impl BatchCommitGuard<'_> {
    #[inline(always)]
    fn record_insert(&mut self) {
        self.inserted += 1;
    }
}

impl Drop for BatchCommitGuard<'_> {
    fn drop(&mut self) {
        *self.live_entity_count += self.inserted;
    }
}

impl World {
    /// Adds an entity in `archetype` without initializing component columns.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component column before the entity can
    /// be observed, migrated, removed, or dropped.
    pub(crate) unsafe fn add_entity(&mut self, archetype: Archetype) -> EntityId {
        let data_index = self.ensure_data_index(archetype);
        let entity = self.allocate_entity();
        self.bump_row_layout_epoch();
        let location = unsafe { self.allocate_storage_row(data_index, entity) };
        self.set_entity_location(
            entity,
            EntityLocation {
                data_index,
                chunk_index: location.chunk_index,
                entity_index: location.entity_index,
            },
        );
        self.live_entity_count += 1;
        entity
    }

    /// Spawns a new entity with the given component bundle.
    ///
    /// Returns the [`EntityId`] of the newly created entity.  The bundle
    /// type is typically a tuple of components:
    ///
    /// ```
    /// # use sky_ecs::World;
    /// # #[derive(Clone, Copy)] struct Pos { x: f32, y: f32 }
    /// # #[derive(Clone, Copy)] struct Vel { x: f32, y: f32 }
    /// # let mut world = World::new();
    /// let entity = world.spawn((Pos { x: 0.0, y: 0.0 }, Vel { x: 1.0, y: 2.0 }));
    /// ```
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityId {
        let (archetype, columns) = B::cached_meta();
        let data_index = self.ensure_data_index(archetype);
        let entity = self.allocate_entity();
        self.bump_row_layout_epoch();
        let location = unsafe { self.allocate_storage_row(data_index, entity) };
        self.set_entity_location(
            entity,
            EntityLocation {
                data_index,
                chunk_index: location.chunk_index,
                entity_index: location.entity_index,
            },
        );

        let chunk = &mut self.data[data_index].chunks[location.chunk_index];
        unsafe {
            bundle.write_fast(chunk, location.entity_index, columns);
        }
        self.live_entity_count += 1;

        entity
    }

    /// Spawns multiple entities from an iterator of bundles.
    ///
    /// More efficient than calling [`spawn`](Self::spawn) in a loop because
    /// the archetype lookup and entity record allocation are amortised.
    pub fn spawn_batch<B: Bundle>(&mut self, bundles: impl IntoIterator<Item = B>) {
        let mut iter = bundles.into_iter();
        let (lower, upper) = iter.size_hint();
        let Some(first) = iter.next() else {
            return;
        };

        let (archetype, columns) = B::cached_meta();
        let data_index = self.ensure_data_index(archetype);
        self.bump_row_layout_epoch();

        let batch_size = upper.filter(|&upper| upper == lower).unwrap_or(lower);

        // The size hint was captured before taking `first`, so `lower` covers
        // the whole batch. Reused slots do not append entity records, while
        // `Vec::reserve` takes an additional count relative to the current len.
        let additional_records = lower.saturating_sub(self.free_entities.len());
        if additional_records > 0 {
            self.entities.reserve(additional_records);
        }

        let mut storage_guard =
            ChunkSetEpochGuard::new(&mut self.data[data_index], &mut self.storage_epochs);
        let mut batch_plan = storage_guard
            .storage_mut()
            .prepare_batch_capacity(batch_size);

        let mut iter = std::iter::once(first).chain(iter).peekable();
        let entities = &mut self.entities;
        let free_entities = &mut self.free_entities;
        let chunk_directory = &mut self.chunk_directory;
        let storage = storage_guard.storage_mut();
        let mut live_count = BatchCommitGuard {
            live_entity_count: &mut self.live_entity_count,
            inserted: 0,
        };

        // Work one chunk at a time. This resolves column starts and checks
        // chunk capacity once per contiguous row span instead of once per
        // entity. A short or panicking iterator is still safe: the sealed
        // Bundle writer initializes every component without invoking user
        // code, and `BatchCommitGuard` records completed rows on unwind.
        while iter.peek().is_some() {
            let guaranteed_remaining = iter.size_hint().0.max(1);
            let chunk_index =
                storage.ensure_planned_batch_tail(&mut batch_plan, guaranteed_remaining);
            let chunk_id = chunk_directory.ensure(
                &mut storage.chunk_ids[chunk_index],
                data_index,
                chunk_index,
            );
            let chunk = &mut storage.chunks[chunk_index];
            let first_entity_index = chunk.entity_count;
            let available = chunk.max_entity_count - first_entity_index;

            let mut cursors = [std::ptr::null_mut(); MAX_COMPONENTS];
            for (cursor, &(component_index, component_size)) in cursors.iter_mut().zip(columns) {
                // SAFETY: `first_entity_index` starts inside the available
                // capacity of this chunk, and the cached component size and
                // column index describe the matching bundle archetype.
                *cursor = unsafe {
                    chunk
                        .column_ptr(component_index)
                        .add(component_size * first_entity_index)
                };
            }

            // Exact and lower-bounded iterators let the common fresh-ID path
            // reserve a whole contiguous row span. Iterator values are still
            // requested one by one, so a short or panicking iterator cannot
            // expose an uninitialized row.
            let fast_rows = if free_entities.is_empty() {
                available.min(iter.size_hint().0)
            } else {
                0
            };
            if fast_rows > 0 {
                assert!(
                    fast_rows <= (u32::MAX as usize).saturating_sub(entities.len()),
                    "entity slot limit exhausted"
                );
                entities.reserve(fast_rows);
                chunk.reserve_entity_slots(fast_rows);
            }

            let first_record_entity_index =
                u32::try_from(first_entity_index).expect("chunk entity index limit exhausted");

            let mut inserted_in_chunk = 0usize;
            for (record_entity_index, row_offset) in (first_record_entity_index..).zip(0..fast_rows)
            {
                let Some(bundle) = iter.next() else {
                    break;
                };

                let index = entities.len() as u32;
                let entity = EntityId::new(index, 0);
                // SAFETY: both vectors reserved `fast_rows` slots before this
                // loop. Each completed iteration consumes exactly one slot.
                unsafe {
                    EntityRecord::append_reserved(
                        entities,
                        EntityRecord::occupied_indices(0, chunk_id, record_entity_index),
                    );
                    let entity_index = chunk.add_entity_reserved_unchecked(entity);
                    debug_assert_eq!(entity_index, first_entity_index + row_offset);
                    bundle.write_fast_cursor(&mut cursors, columns);
                }
                live_count.record_insert();
                inserted_in_chunk += 1;
            }

            for row_offset in inserted_in_chunk..available {
                let Some(bundle) = iter.next() else {
                    break;
                };

                let route = EntityRoute {
                    chunk_id,
                    entity_index: first_entity_index + row_offset,
                };
                let entity = Self::allocate_entity_at_location(entities, free_entities, route);
                // SAFETY: this loop is bounded by the tail's available row
                // count, and the sealed Bundle writer below initializes every
                // component before the row can be observed.
                let entity_index = unsafe { chunk.add_entity_unchecked(entity) };
                debug_assert_eq!(entity_index, route.entity_index);

                // SAFETY: the cursors were initialized from this chunk's
                // cached bundle columns at `first_entity_index` and advance by
                // exactly one component slot after every completed row.
                unsafe {
                    bundle.write_fast_cursor(&mut cursors, columns);
                }
                live_count.record_insert();
            }
        }
    }

    pub(crate) fn spawn_dynamic_values(
        &mut self,
        values: &mut [crate::ecs::dynamic::ErasedComponentValue],
    ) -> Result<EntityId, crate::ecs::dynamic::DynamicSpawnError> {
        crate::ecs::dynamic::validate_dynamic_values(values)?;

        let mut builder = create_archetype();
        for value in values.iter() {
            builder = builder.add_component(value.component);
        }
        let archetype = builder.build();

        let entity = unsafe { self.add_entity(archetype) };
        let location = self
            .entity_location(entity)
            .expect("fresh dynamic entity must have a location");
        let chunk = &mut self.data[location.data_index].chunks[location.chunk_index];

        for value in values {
            let component_index = archetype
                .query_component_index(&value.component)
                .expect("dynamic component must exist in target archetype");
            let ptr = unsafe {
                chunk
                    .column_ptr(component_index)
                    .add(location.entity_index * value.component.size)
            };
            value.value.write(ptr);
        }

        Ok(entity)
    }

    /// Returns `true` if `entity` is alive in this world.
    pub fn contains(&self, entity: EntityId) -> bool {
        self.entity_location(entity).is_some()
    }

    /// Iterates all live entities in dense storage order.
    ///
    /// Entity IDs are runtime-local handles. Systems that need persistent
    /// document identity should store their own stable component alongside
    /// these IDs.
    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.data
            .iter()
            .flat_map(|data| data.chunks.iter())
            .flat_map(|chunk| chunk.entities().iter().copied())
    }

    /// Returns `true` if `entity` is alive and has component `T`.
    #[inline(always)]
    pub fn has<T: 'static>(&self, entity: EntityId) -> bool {
        let Some(location) = self.entity_location(entity) else {
            return false;
        };

        self.data[location.data_index]
            .archetype
            .has_component(&component_type::<T>())
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
        let mut drop_panic = None;

        {
            let chunk = &self.data[location.data_index].chunks[location.chunk_index];
            // Safety: `location` identifies a live row. Every droppable
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

        self.finish_chunk_removal(location.data_index, removal);

        if let Some(payload) = drop_panic {
            std::panic::resume_unwind(payload);
        }

        true
    }

    /// Returns a shared reference to component `T` on `entity`.
    ///
    /// Returns `None` if the entity is dead or does not have `T`.
    #[inline(always)]
    pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T> {
        let location = self.entity_location(entity)?;
        let data = &self.data[location.data_index];
        let chunk = &data.chunks[location.chunk_index];
        let component_index = chunk
            .archetype
            .query_component_index(&component_type::<T>())?;

        Some(unsafe {
            let ptr = chunk
                .column_ptr(component_index)
                .add(location.entity_index * std::mem::size_of::<T>());
            &*(ptr as *const T)
        })
    }

    /// Returns an exclusive reference to component `T` on `entity`.
    ///
    /// Returns `None` if the entity is dead or does not have `T`.
    #[inline(always)]
    pub fn get_mut<T: 'static>(&mut self, entity: EntityId) -> Option<&mut T> {
        let location = self.entity_location(entity)?;
        let data = &mut self.data[location.data_index];
        let chunk = &mut data.chunks[location.chunk_index];
        let component_index = chunk
            .archetype
            .query_component_index(&component_type::<T>())?;

        Some(unsafe {
            let ptr = chunk
                .column_ptr(component_index)
                .add(location.entity_index * std::mem::size_of::<T>());
            &mut *(ptr as *mut T)
        })
    }
}

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
        self.bump_storage_epoch();
        let location = unsafe { self.data[data_index].add_entity(entity) };
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
        self.bump_storage_epoch();
        let location = unsafe { self.data[data_index].add_entity(entity) };
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
        self.bump_storage_epoch();

        let batch_size = upper.filter(|&upper| upper == lower).unwrap_or(lower);

        // The size hint was captured before taking `first`, so `lower` covers
        // the whole batch. Reused slots do not append entity records, while
        // `Vec::reserve` takes an additional count relative to the current len.
        let additional_records = lower.saturating_sub(self.free_entities.len());
        if additional_records > 0 {
            self.entities.reserve(additional_records);
        }

        self.data[data_index].prepare_batch_capacity(batch_size);

        let mut iter = std::iter::once(first).chain(iter).peekable();
        let entities = &mut self.entities;
        let free_entities = &mut self.free_entities;
        let storage = &mut self.data[data_index];
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
            let chunk_index = storage.ensure_batch_tail(guaranteed_remaining);
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

            for row_offset in 0..available {
                let Some(bundle) = iter.next() else {
                    break;
                };

                let location = EntityLocation {
                    data_index,
                    chunk_index,
                    entity_index: first_entity_index + row_offset,
                };
                let entity = Self::allocate_entity_at_location(entities, free_entities, location);
                // SAFETY: this loop is bounded by the tail's available row
                // count, and the sealed Bundle writer below initializes every
                // component before the row can be observed.
                let entity_index = unsafe { chunk.add_entity_unchecked(entity) };
                debug_assert_eq!(entity_index, location.entity_index);

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
        for (index, value) in values.iter().enumerate() {
            for other in &values[(index + 1)..] {
                if value.component.id() == other.component.id() {
                    return Err(crate::ecs::dynamic::DynamicSpawnError::DuplicateComponent {
                        component: value.component,
                    });
                }
            }
        }

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

        self.bump_storage_epoch();
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

        let moved = self.data[location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: location.chunk_index,
            entity_index: location.entity_index,
        });

        let record = &mut self.entities[entity.index() as usize];
        record.clear_location();
        if let Some(next_generation) = record.generation.checked_add(1) {
            record.generation = next_generation;
            self.free_entities.push(entity.index());
        }
        self.live_entity_count -= 1;

        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: location.data_index,
                    chunk_index: moved_location.chunk_index,
                    entity_index: moved_location.entity_index,
                },
            );
        }

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

    /// Creates a read-only accessor for repeated random access to component `T`.
    ///
    /// Construction resolves `T` once for every archetype and caches typed
    /// views of matching chunk columns. This is useful when a hot loop performs
    /// many lookups by [`EntityId`]. For an occasional lookup, use
    /// [`World::get`] directly.
    ///
    /// The accessor holds a shared borrow of this world, so structural changes
    /// cannot occur while it remains in use.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// let positions = world.accessor::<Position>();
    ///
    /// assert_eq!(positions.get(entity), Some(&Position(1.0, 2.0)));
    /// ```
    ///
    /// Structural mutation is rejected while the accessor is still used:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// let positions = world.accessor::<Position>();
    /// world.spawn((Position(3.0, 4.0),));
    /// let _ = positions.get(entity);
    /// ```
    #[inline]
    pub fn accessor<T: 'static>(&self) -> ComponentAccessor<'_, T> {
        ComponentAccessor::new(self)
    }

    /// Creates an exclusive accessor for repeated random updates to component `T`.
    ///
    /// Like [`World::accessor`], construction resolves matching component
    /// columns before the hot loop. The accessor exclusively borrows the world,
    /// and each component reference remains tied to one mutable accessor borrow.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// {
    ///     let mut positions = world.accessor_mut::<Position>();
    ///     positions.get_mut(entity).unwrap().0 += 3.0;
    /// }
    /// assert_eq!(world.get::<Position>(entity).unwrap().0, 4.0);
    /// ```
    ///
    /// Mutable references from the same accessor cannot overlap:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let first = world.spawn((Position(1.0, 2.0),));
    /// let second = world.spawn((Position(3.0, 4.0),));
    /// let mut positions = world.accessor_mut::<Position>();
    /// let first_position = positions.get_mut(first).unwrap();
    /// let second_position = positions.get_mut(second).unwrap();
    /// first_position.0 += second_position.0;
    /// ```
    #[inline]
    pub fn accessor_mut<T: 'static>(&mut self) -> ComponentAccessorMut<'_, T> {
        ComponentAccessorMut::new(self)
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

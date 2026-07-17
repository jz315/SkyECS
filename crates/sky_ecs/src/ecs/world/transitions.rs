use super::*;

impl World {
    fn archetype_with_component(base: Archetype, component: ComponentType) -> Archetype {
        let mut builder = create_archetype();
        for existing in &base.components {
            builder = builder.add_component(*existing);
        }
        builder.add_component(component).build()
    }

    fn archetype_without_component(base: Archetype, component: ComponentType) -> Option<Archetype> {
        if !base.has_component(&component) {
            return None;
        }

        let mut builder = create_archetype();
        for existing in &base.components {
            if existing.id() != component.id() {
                builder = builder.add_component(*existing);
            }
        }

        Some(builder.build())
    }

    /// Build copy-span descriptors for transitioning entity data from `source`
    /// to `target`. Each span stores source/target component indices rather
    /// than byte offsets because chunks of the same archetype may use different
    /// block sizes. Components whose type-id appears in `skip_component_ids`
    /// are excluded.
    fn build_copy_spans(
        source: &ArchetypeStorage,
        target: &ArchetypeStorage,
        skip_component_ids: &[usize],
    ) -> SmallVec<[CopySpan; 8]> {
        let mut spans = SmallVec::new();

        for (source_index, component) in source.archetype.components.iter().enumerate() {
            if component.size == 0 || skip_component_ids.iter().any(|id| *id == component.id()) {
                continue;
            }

            let Some(target_index) = target.archetype.query_component_index(component) else {
                continue;
            };
            spans.push(CopySpan {
                source_component: source_index as u8,
                target_component: target_index as u8,
                component_size: u32::try_from(component.size)
                    .expect("component exceeds supported chunk offset range"),
            });
        }

        spans
    }

    fn transition_plan(
        &mut self,
        source_data_index: usize,
        component: ComponentType,
        add: bool,
    ) -> Option<NonNull<TransitionPlan>> {
        let base = self.data[source_data_index].archetype;
        let key = TransitionKey {
            archetype: base,
            component_id: component.id(),
            add,
        };

        if let Some((cached_key, plan)) = self.last_transition {
            if cached_key == key {
                return Some(plan);
            }
        }

        if let Some(plan) = self.transitions.get(&key) {
            let plan = NonNull::from(plan.as_ref());
            self.last_transition = Some((key, plan));
            return Some(plan);
        }

        let plan = if add {
            if base.has_component(&component) {
                return None;
            }

            let target_archetype = Self::archetype_with_component(base, component);
            let target_data_index = self.ensure_data_index(target_archetype);
            let target_component_index =
                target_archetype.query_component_index(&component).unwrap();
            TransitionPlan {
                copy_spans: Self::build_copy_spans(
                    &self.data[source_data_index],
                    &self.data[target_data_index],
                    &[],
                ),
                target_component_index: Some(target_component_index as u8),
                target_data_index,
            }
        } else {
            let target_archetype = Self::archetype_without_component(base, component)?;
            let target_data_index = self.ensure_data_index(target_archetype);
            TransitionPlan {
                copy_spans: Self::build_copy_spans(
                    &self.data[source_data_index],
                    &self.data[target_data_index],
                    &[],
                ),
                target_component_index: None,
                target_data_index,
            }
        };

        let entry = self
            .transitions
            .entry(key)
            .or_insert_with(|| Box::new(plan));
        let plan = NonNull::from(entry.as_ref());
        self.last_transition = Some((key, plan));
        Some(plan)
    }

    fn component_command_transition_plan(
        &mut self,
        source_data_index: usize,
        commands: &[PendingComponentEntry],
    ) -> NonNull<ComponentCommandTransitionPlan> {
        let source_archetype = self.data[source_data_index].archetype;
        let mut operations = commands
            .iter()
            .map(|entry| {
                (
                    entry.component.id(),
                    matches!(entry.command, PendingComponentCommand::Insert(_)),
                )
            })
            .collect::<SmallVec<[(usize, bool); 4]>>();
        operations.sort_unstable();
        let key = ComponentCommandTransitionKey {
            archetype: source_archetype,
            operations,
        };

        if let Some(plan) = self.component_command_transitions.get(&key) {
            return NonNull::from(plan.as_ref());
        }

        let mut target_builder = create_archetype();
        for source_component in &source_archetype.components {
            let removed = commands.iter().any(|entry| {
                entry.component.id() == source_component.id()
                    && matches!(entry.command, PendingComponentCommand::Remove)
            });
            if !removed {
                target_builder = target_builder.add_component(*source_component);
            }
        }
        for entry in commands {
            if matches!(entry.command, PendingComponentCommand::Insert(_))
                && !source_archetype.has_component(&entry.component)
            {
                target_builder = target_builder.add_component(entry.component);
            }
        }

        let target_archetype = target_builder.build();
        debug_assert_ne!(target_archetype, source_archetype);
        let target_data_index = self.ensure_data_index(target_archetype);
        let replaced_component_ids = commands
            .iter()
            .filter_map(|entry| {
                matches!(entry.command, PendingComponentCommand::Insert(_))
                    .then_some(entry.component.id())
            })
            .collect::<SmallVec<[usize; MAX_COMPONENTS]>>();
        let plan = ComponentCommandTransitionPlan {
            target_archetype,
            target_data_index,
            copy_spans: Self::build_copy_spans(
                &self.data[source_data_index],
                &self.data[target_data_index],
                &replaced_component_ids,
            ),
        };

        let plan = self
            .component_command_transitions
            .entry(key)
            .or_insert_with(|| Box::new(plan));
        NonNull::from(plan.as_ref())
    }

    fn copy_components_with_spans(
        source: &Chunk,
        source_entity_index: usize,
        target: &mut Chunk,
        target_entity_index: usize,
        spans: &[CopySpan],
    ) {
        for &CopySpan {
            source_component,
            target_component,
            component_size,
        } in spans
        {
            let source_component_index = source_component as usize;
            let target_component_index = target_component as usize;
            let component_size = component_size as usize;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    source
                        .column_ptr(source_component_index)
                        .add(source_entity_index * component_size),
                    target
                        .column_ptr(target_component_index)
                        .add(target_entity_index * component_size),
                    component_size,
                );
            }
        }
    }

    /// Runs one erased component destructor while retaining at most the first
    /// panic payload. The caller can then restore a valid ownership state
    /// before resuming that panic.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a live value of `component`, and the caller must
    /// treat that value as dropped after this function returns, including when
    /// its destructor panicked.
    unsafe fn drop_component_catching(
        component: ComponentType,
        ptr: *mut u8,
        first_panic: &mut Option<Box<dyn std::any::Any + Send>>,
    ) {
        let Some(drop_fn) = component.drop_fn() else {
            return;
        };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            drop_fn(ptr);
        }));
        if let Err(payload) = result {
            if first_panic.is_none() {
                *first_panic = Some(payload);
            } else {
                // A later panic payload must not be dropped during recovery:
                // a user-defined payload destructor could itself panic.
                std::mem::forget(payload);
            }
        }
    }

    /// Drops every droppable component in one occupied row while retaining at
    /// most the first panic payload.
    ///
    /// # Safety
    ///
    /// `entity_index` must identify a live row in `chunk`. The caller must
    /// remove or mark that row uninitialized before any captured panic resumes.
    pub(super) unsafe fn drop_entity_components_catching(
        chunk: &Chunk,
        entity_index: usize,
        first_panic: &mut Option<Box<dyn std::any::Any + Send>>,
    ) {
        for &component_index in &chunk.archetype.drop_component_indices {
            let component = chunk.archetype.components[component_index as usize];
            let ptr = unsafe {
                chunk
                    .column_ptr(component_index as usize)
                    .add(entity_index * component.size)
            };
            unsafe {
                Self::drop_component_catching(component, ptr, first_panic);
            }
        }
    }

    /// Adds or overwrites component `T` on `entity`.
    ///
    /// If the entity already has `T`, the old value is dropped and replaced
    /// in-place (no archetype migration).  If the entity does not have `T`,
    /// it is migrated to a new archetype that includes `T`.
    ///
    /// Returns `false` if the entity does not exist.
    /// If the old component's destructor panics while overwriting, the
    /// replacement is installed before that panic resumes.
    pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) -> bool {
        let Some(source_location) = self.entity_location(entity) else {
            return false;
        };

        let component_ty = component_type::<T>();
        let source_archetype = self.data[source_location.data_index].archetype;

        // Overwrite path: entity already has this component.
        if source_archetype.has_component(&component_ty) {
            let component_index = source_archetype
                .query_component_index(&component_ty)
                .unwrap();
            let chunk =
                &mut self.data[source_location.data_index].chunks[source_location.chunk_index];
            let mut drop_panic = None;
            unsafe {
                let ptr = chunk
                    .column_ptr(component_index)
                    .add(source_location.entity_index * std::mem::size_of::<T>())
                    as *mut T;
                // Safety: `ptr` points to a live `T`. The replacement is
                // installed even if destroying the old value panics, so the
                // occupied slot is always initialized when the panic resumes.
                Self::drop_component_catching(component_ty, ptr.cast::<u8>(), &mut drop_panic);
                std::ptr::write(ptr, component);
            }
            if let Some(payload) = drop_panic {
                std::panic::resume_unwind(payload);
            }
            return true;
        }

        let plan = self
            .transition_plan(source_location.data_index, component_ty, true)
            .expect("adding a missing component must produce a transition plan");
        let plan = unsafe { plan.as_ref() };
        let target_data_index = plan.target_data_index;
        self.bump_storage_epoch();
        let target_location = unsafe { self.data[target_data_index].add_entity(entity) };

        {
            let (source_chunk, target_chunk) = if source_location.data_index < target_data_index {
                let (left, right) = self.data.split_at_mut(target_data_index);
                (
                    &left[source_location.data_index].chunks[source_location.chunk_index],
                    &mut right[0].chunks[target_location.chunk_index],
                )
            } else {
                let (left, right) = self.data.split_at_mut(source_location.data_index);
                (
                    &right[0].chunks[source_location.chunk_index],
                    &mut left[target_data_index].chunks[target_location.chunk_index],
                )
            };

            // Bitwise-copy existing columns to the new archetype chunk.
            // This is a semantic move: the source slot should NOT be dropped
            // for these columns.
            Self::copy_components_with_spans(
                source_chunk,
                source_location.entity_index,
                target_chunk,
                target_location.entity_index,
                &plan.copy_spans,
            );

            let target_component_index = plan
                .target_component_index
                .expect("add transition plans must include the inserted component offset");
            unsafe {
                let ptr = target_chunk
                    .column_ptr(target_component_index as usize)
                    .add(target_location.entity_index * std::mem::size_of::<T>());
                std::ptr::write(ptr as *mut T, component);
            }
        }

        self.set_entity_location(
            entity,
            EntityLocation {
                data_index: target_data_index,
                chunk_index: target_location.chunk_index,
                entity_index: target_location.entity_index,
            },
        );

        // Source entity data was bitwise-moved to target; the swap-remove
        // here only rearranges the source chunk — no drops needed.
        let moved = self.data[source_location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: source_location.chunk_index,
            entity_index: source_location.entity_index,
        });

        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: source_location.data_index,
                    chunk_index: moved_location.chunk_index,
                    entity_index: moved_location.entity_index,
                },
            );
        }

        true
    }

    /// Removes component `T` from `entity`, dropping it.
    ///
    /// The entity is migrated to a smaller archetype.  Returns `false` if
    /// the entity does not exist or does not have `T`.
    /// If the removed component's destructor panics, migration and location
    /// repair finish before that panic resumes.
    pub fn remove<T: 'static>(&mut self, entity: EntityId) -> bool {
        let Some(source_location) = self.entity_location(entity) else {
            return false;
        };

        let component_ty = component_type::<T>();
        let Some(plan) = self.transition_plan(source_location.data_index, component_ty, false)
        else {
            return false;
        };
        let plan = unsafe { plan.as_ref() };

        let target_data_index = plan.target_data_index;
        self.bump_storage_epoch();
        let target_location = unsafe { self.data[target_data_index].add_entity(entity) };
        let mut drop_panic = None;

        {
            let (source_chunk, target_chunk) = if source_location.data_index < target_data_index {
                let (left, right) = self.data.split_at_mut(target_data_index);
                (
                    &left[source_location.data_index].chunks[source_location.chunk_index],
                    &mut right[0].chunks[target_location.chunk_index],
                )
            } else {
                let (left, right) = self.data.split_at_mut(source_location.data_index);
                (
                    &right[0].chunks[source_location.chunk_index],
                    &mut left[target_data_index].chunks[target_location.chunk_index],
                )
            };

            // Bitwise-copy kept columns to target (semantic move).
            Self::copy_components_with_spans(
                source_chunk,
                source_location.entity_index,
                target_chunk,
                target_location.entity_index,
                &plan.copy_spans,
            );

            // Drop the removed component column from the source entity.
            // Safety: source_location is valid and this column is being
            // discarded (not copied to the target archetype).
            if let Some(removed_component_index) =
                source_chunk.archetype.query_component_index(&component_ty)
            {
                let ptr = unsafe {
                    source_chunk
                        .column_ptr(removed_component_index)
                        .add(source_location.entity_index * component_ty.size)
                };
                // Safety: this value is removed rather than moved. The source
                // row is compacted before a captured panic is resumed.
                unsafe {
                    Self::drop_component_catching(component_ty, ptr, &mut drop_panic);
                }
            }
        }

        self.set_entity_location(
            entity,
            EntityLocation {
                data_index: target_data_index,
                chunk_index: target_location.chunk_index,
                entity_index: target_location.entity_index,
            },
        );

        // Source entity data was bitwise-moved (kept columns) and dropped
        // (removed column); the swap-remove only rearranges the chunk.
        let moved = self.data[source_location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: source_location.chunk_index,
            entity_index: source_location.entity_index,
        });

        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: source_location.data_index,
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

    fn insert_dynamic(
        &mut self,
        entity: EntityId,
        component: ComponentType,
        value: &mut super::erased_value::InsertValue,
    ) -> bool {
        let Some(source_location) = self.entity_location(entity) else {
            return false;
        };

        let source_archetype = self.data[source_location.data_index].archetype;

        if source_archetype.has_component(&component) {
            let component_index = source_archetype.query_component_index(&component).unwrap();
            let data = &mut self.data[source_location.data_index];
            let chunk = &mut data.chunks[source_location.chunk_index];
            let ptr = unsafe {
                chunk
                    .column_ptr(component_index)
                    .add(source_location.entity_index * component.size)
            };
            let mut drop_panic = None;
            // Safety: this is the live component value being overwritten.
            unsafe { Self::drop_component_catching(component, ptr, &mut drop_panic) };
            value.write(ptr);
            if let Some(payload) = drop_panic {
                std::panic::resume_unwind(payload);
            }
            return true;
        }

        let plan = self
            .transition_plan(source_location.data_index, component, true)
            .expect("adding a missing component must produce a transition plan");
        let plan = unsafe { plan.as_ref() };
        let target_data_index = plan.target_data_index;
        self.bump_storage_epoch();
        let target_location = unsafe { self.data[target_data_index].add_entity(entity) };

        {
            let (source_chunk, target_chunk) = if source_location.data_index < target_data_index {
                let (left, right) = self.data.split_at_mut(target_data_index);
                (
                    &left[source_location.data_index].chunks[source_location.chunk_index],
                    &mut right[0].chunks[target_location.chunk_index],
                )
            } else {
                let (left, right) = self.data.split_at_mut(source_location.data_index);
                (
                    &right[0].chunks[source_location.chunk_index],
                    &mut left[target_data_index].chunks[target_location.chunk_index],
                )
            };

            Self::copy_components_with_spans(
                source_chunk,
                source_location.entity_index,
                target_chunk,
                target_location.entity_index,
                &plan.copy_spans,
            );

            let target_component_index = plan
                .target_component_index
                .expect("add transition plans must include the inserted component offset");
            let ptr = unsafe {
                target_chunk
                    .column_ptr(target_component_index as usize)
                    .add(target_location.entity_index * component.size)
            };
            value.write(ptr);
        }

        self.set_entity_location(
            entity,
            EntityLocation {
                data_index: target_data_index,
                chunk_index: target_location.chunk_index,
                entity_index: target_location.entity_index,
            },
        );

        let moved = self.data[source_location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: source_location.chunk_index,
            entity_index: source_location.entity_index,
        });
        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: source_location.data_index,
                    chunk_index: moved_location.chunk_index,
                    entity_index: moved_location.entity_index,
                },
            );
        }

        true
    }

    fn remove_dynamic(&mut self, entity: EntityId, component: ComponentType) -> bool {
        let Some(source_location) = self.entity_location(entity) else {
            return false;
        };

        let Some(plan) = self.transition_plan(source_location.data_index, component, false) else {
            return false;
        };
        let plan = unsafe { plan.as_ref() };
        let target_data_index = plan.target_data_index;
        self.bump_storage_epoch();
        let target_location = unsafe { self.data[target_data_index].add_entity(entity) };
        let mut drop_panic = None;

        {
            let (source_chunk, target_chunk) = if source_location.data_index < target_data_index {
                let (left, right) = self.data.split_at_mut(target_data_index);
                (
                    &left[source_location.data_index].chunks[source_location.chunk_index],
                    &mut right[0].chunks[target_location.chunk_index],
                )
            } else {
                let (left, right) = self.data.split_at_mut(source_location.data_index);
                (
                    &right[0].chunks[source_location.chunk_index],
                    &mut left[target_data_index].chunks[target_location.chunk_index],
                )
            };

            Self::copy_components_with_spans(
                source_chunk,
                source_location.entity_index,
                target_chunk,
                target_location.entity_index,
                &plan.copy_spans,
            );

            if let Some(removed_component_index) =
                source_chunk.archetype.query_component_index(&component)
            {
                let ptr = unsafe {
                    source_chunk
                        .column_ptr(removed_component_index)
                        .add(source_location.entity_index * component.size)
                };
                // Safety: this value is removed rather than moved and the
                // source row is compacted before a captured panic resumes.
                unsafe {
                    Self::drop_component_catching(component, ptr, &mut drop_panic);
                }
            }
        }

        self.set_entity_location(
            entity,
            EntityLocation {
                data_index: target_data_index,
                chunk_index: target_location.chunk_index,
                entity_index: target_location.entity_index,
            },
        );

        let moved = self.data[source_location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: source_location.chunk_index,
            entity_index: source_location.entity_index,
        });
        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: source_location.data_index,
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

    /// Applies the coalesced final component state for one entity.
    ///
    /// If the final component set differs from the source archetype, all
    /// additions and removals are committed through one source-to-target
    /// migration. Insertions of components that already exist remain
    /// overwrites: the old value is dropped and the queued value takes its
    /// place. A missing removal remains a no-op.
    pub(in crate::ecs) fn apply_component_commands(
        &mut self,
        entity: EntityId,
        commands: &mut [PendingComponentEntry],
    ) -> bool {
        if let [entry] = commands {
            return match &mut entry.command {
                PendingComponentCommand::Insert(value) => {
                    self.insert_dynamic(entity, entry.component, value)
                }
                PendingComponentCommand::Remove => self.remove_dynamic(entity, entry.component),
            };
        }

        let Some(source_location) = self.entity_location(entity) else {
            return false;
        };

        let source_archetype = self.data[source_location.data_index].archetype;
        let changes_archetype = commands.iter().any(|entry| match &entry.command {
            PendingComponentCommand::Insert(_) => !source_archetype.has_component(&entry.component),
            PendingComponentCommand::Remove => source_archetype.has_component(&entry.component),
        });

        // A batch with no set changes contains only overwrites and removals of
        // components that were already absent. Apply it directly without an
        // archetype-cache lookup or a storage mutation.
        if !changes_archetype {
            let mut drop_panic = None;
            {
                let data = &mut self.data[source_location.data_index];
                let chunk = &mut data.chunks[source_location.chunk_index];

                for entry in commands {
                    let PendingComponentCommand::Insert(value) = &mut entry.command else {
                        continue;
                    };
                    let component_index = source_archetype
                        .query_component_index(&entry.component)
                        .expect(
                            "an in-place command insertion must overwrite an existing component",
                        );
                    let ptr = unsafe {
                        chunk
                            .column_ptr(component_index)
                            .add(source_location.entity_index * entry.component.size)
                    };
                    // Safety: this is the live source value being overwritten.
                    unsafe {
                        Self::drop_component_catching(entry.component, ptr, &mut drop_panic);
                    }
                    value.write(ptr);
                }
            }

            if let Some(payload) = drop_panic {
                std::panic::resume_unwind(payload);
            }
            return true;
        }

        let plan = self.component_command_transition_plan(source_location.data_index, commands);
        // Safety: transition plans are boxed and remain at stable addresses
        // until World::clear. No transition-cache mutation occurs while this
        // reference is used.
        let plan = unsafe { plan.as_ref() };
        let target_archetype = plan.target_archetype;
        let target_data_index = plan.target_data_index;
        let replaced_component_ids = commands
            .iter()
            .filter_map(|entry| {
                matches!(entry.command, PendingComponentCommand::Insert(_))
                    .then_some(entry.component.id())
            })
            .collect::<SmallVec<[usize; MAX_COMPONENTS]>>();

        self.bump_storage_epoch();
        let target_location = unsafe { self.data[target_data_index].add_entity(entity) };
        let mut drop_panic = None;

        {
            let (source_chunk, target_chunk) = if source_location.data_index < target_data_index {
                let (left, right) = self.data.split_at_mut(target_data_index);
                (
                    &left[source_location.data_index].chunks[source_location.chunk_index],
                    &mut right[0].chunks[target_location.chunk_index],
                )
            } else {
                let (left, right) = self.data.split_at_mut(source_location.data_index);
                (
                    &right[0].chunks[source_location.chunk_index],
                    &mut left[target_data_index].chunks[target_location.chunk_index],
                )
            };

            // Components that survive unchanged are bitwise-moved into their
            // target columns. Their source bytes must not be dropped.
            Self::copy_components_with_spans(
                source_chunk,
                source_location.entity_index,
                target_chunk,
                target_location.entity_index,
                &plan.copy_spans,
            );

            // Every queued insertion owns the final value for its component,
            // regardless of whether it is an addition or an overwrite.
            for entry in commands.iter_mut() {
                let PendingComponentCommand::Insert(value) = &mut entry.command else {
                    continue;
                };
                let target_component_index = target_archetype
                    .query_component_index(&entry.component)
                    .expect("the final archetype must contain every inserted component");
                let ptr = unsafe {
                    target_chunk
                        .column_ptr(target_component_index)
                        .add(target_location.entity_index * entry.component.size)
                };
                value.write(ptr);
            }

            // Removed values and overwritten source values were not moved to
            // the target. Drop each of them exactly once before discarding the
            // source row.
            for (source_component_index, source_component) in
                source_archetype.components.iter().enumerate()
            {
                let survives_unchanged = target_archetype.has_component(source_component)
                    && !replaced_component_ids.contains(&source_component.id());
                if !survives_unchanged {
                    let ptr = unsafe {
                        source_chunk
                            .column_ptr(source_component_index)
                            .add(source_location.entity_index * source_component.size)
                    };
                    // Safety: this source value was deliberately excluded from
                    // the semantic move. The source row is compacted before a
                    // captured panic resumes.
                    unsafe {
                        Self::drop_component_catching(*source_component, ptr, &mut drop_panic);
                    }
                }
            }
        }

        self.set_entity_location(
            entity,
            EntityLocation {
                data_index: target_data_index,
                chunk_index: target_location.chunk_index,
                entity_index: target_location.entity_index,
            },
        );

        // Every source component was either semantically moved or explicitly
        // dropped above, so compact the source row without running destructors.
        let moved = self.data[source_location.data_index].remove_entity(ChunkEntityLocation {
            chunk_index: source_location.chunk_index,
            entity_index: source_location.entity_index,
        });

        if let Some((moved_entity, moved_location)) = moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index: source_location.data_index,
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
}

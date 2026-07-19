use super::*;

impl World {
    /// Spawns entities from component columns that already use a structure of
    /// arrays layout.
    ///
    /// Every column must have the same length. Component values are moved into
    /// the World without cloning; on success each source `Vec` is empty but
    /// retains its allocation for reuse. If lengths differ, the World and all
    /// source columns remain unchanged.
    ///
    /// This path is intended for bulk import, deserialization, simulation
    /// output, and other producers that naturally build separate component
    /// arrays. [`spawn_batch`](Self::spawn_batch) remains the more convenient
    /// choice when data is naturally produced one entity at a time.
    ///
    /// # Example
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// #[derive(Clone, Copy)]
    /// struct Position(f32);
    /// #[derive(Clone, Copy)]
    /// struct Velocity(f32);
    ///
    /// let mut columns = (
    ///     vec![Position(0.0), Position(1.0)],
    ///     vec![Velocity(2.0), Velocity(3.0)],
    /// );
    /// let position_capacity = columns.0.capacity();
    ///
    /// let mut world = World::new();
    /// world.spawn_columns(&mut columns).unwrap();
    ///
    /// assert_eq!(world.query::<(&Position, &Velocity)>().count(), 2);
    /// assert!(columns.0.is_empty());
    /// assert_eq!(columns.0.capacity(), position_capacity);
    /// ```
    pub fn spawn_columns<C: ColumnBundle>(
        &mut self,
        columns: &mut C,
    ) -> Result<(), ColumnLengthMismatch> {
        let entity_count = columns.row_count()?;
        if entity_count == 0 {
            return Ok(());
        }

        let (archetype, component_columns) = C::cached_meta();
        let data_index = self.ensure_data_index(archetype);
        let new_entity_count = entity_count.saturating_sub(self.free_entities.len());
        assert!(
            new_entity_count <= (u32::MAX as usize).saturating_sub(self.entities.len()),
            "entity slot limit exhausted"
        );
        self.entities.reserve(new_entity_count);

        self.bump_storage_epoch();
        let storage = &mut self.data[data_index];
        let spans = storage.reserve_exact_batch_spans(entity_count);
        let record_data_index = u32::try_from(data_index)
            .ok()
            .filter(|&index| index != u32::MAX)
            .expect("World storage index limit exhausted");
        for span in &spans {
            u32::try_from(span.chunk_index).expect("chunk index limit exhausted");
            u32::try_from(span.first_entity_index + span.entity_count - 1)
                .expect("chunk entity index limit exhausted");
        }

        // SAFETY: `reserve_exact_batch_spans` returns logically unoccupied
        // spans totaling exactly `entity_count`, and this metadata belongs to
        // the same column tuple and archetype.
        unsafe {
            columns.move_into(storage, &spans, component_columns);
        }

        let entities = &mut self.entities;
        let free_entities = &mut self.free_entities;

        for span in spans {
            let record_chunk_index = span.chunk_index as u32;
            let chunk = &mut storage.chunks[span.chunk_index];
            debug_assert_eq!(chunk.entity_count, span.first_entity_index);

            let reused_in_span = span.entity_count.min(free_entities.len());
            for row_offset in 0..reused_in_span {
                let index = free_entities.pop().unwrap();
                let record = &mut entities[index as usize];
                let entity = EntityId::new(index, record.generation);
                let record_entity_index = (span.first_entity_index + row_offset) as u32;
                record.set_location_indices(
                    record_data_index,
                    record_chunk_index,
                    record_entity_index,
                );
                unsafe {
                    chunk.add_entity_reserved_unchecked(entity);
                }
            }

            for row_offset in reused_in_span..span.entity_count {
                let index = entities.len() as u32;
                let entity_index = span.first_entity_index + row_offset;
                let record_entity_index = entity_index as u32;
                unsafe {
                    EntityRecord::append_reserved(
                        entities,
                        EntityRecord::occupied_indices(
                            0,
                            record_data_index,
                            record_chunk_index,
                            record_entity_index,
                        ),
                    );
                    let actual_entity_index =
                        chunk.add_entity_reserved_unchecked(EntityId::new(index, 0));
                    debug_assert_eq!(actual_entity_index, entity_index);
                }
            }
            debug_assert_eq!(
                chunk.entity_count,
                span.first_entity_index + span.entity_count
            );
        }

        self.live_entity_count += entity_count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::World;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Health(f32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Damage(f32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Marker;

    struct DropTracked {
        drops: Arc<AtomicUsize>,
        value: usize,
        _padding: [u8; 1_000],
    }

    impl Drop for DropTracked {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn preserves_rows_across_component_columns() {
        let positions: Vec<_> = (0..10_000)
            .map(|value| Position {
                x: value as f32,
                y: value as f32 + 0.5,
            })
            .collect();
        let velocities: Vec<_> = (0..10_000)
            .map(|value| Velocity {
                x: value as f32 * 2.0,
                y: value as f32 * 3.0,
            })
            .collect();
        let health: Vec<_> = (0..10_000)
            .map(|value| Health(value as f32 + 4.0))
            .collect();
        let damage: Vec<_> = (0..10_000)
            .map(|value| Damage(value as f32 + 5.0))
            .collect();
        let mut columns = (positions, velocities, health, damage);
        let position_capacity = columns.0.capacity();
        let mut world = World::new();

        world.spawn_columns(&mut columns).unwrap();

        assert!(columns.0.is_empty());
        assert_eq!(columns.0.capacity(), position_capacity);
        assert_eq!(world.entity_count(), 10_000);
        let mut count = 0;
        world
            .query::<(&Position, &Velocity, &Health, &Damage)>()
            .for_each(|(position, velocity, health, damage)| {
                let value = position.x;
                assert_eq!(position.y, value + 0.5);
                assert_eq!(velocity.x, value * 2.0);
                assert_eq!(velocity.y, value * 3.0);
                assert_eq!(health.0, value + 4.0);
                assert_eq!(damage.0, value + 5.0);
                count += 1;
            });
        assert_eq!(count, 10_000);
    }

    #[test]
    fn rejects_mismatched_lengths_before_mutating_inputs_or_world() {
        let mut columns = (
            vec![Position { x: 1.0, y: 2.0 }; 2],
            vec![Velocity { x: 3.0, y: 4.0 }],
            vec![Health(5.0); 2],
            vec![Damage(6.0); 2],
        );
        let mut world = World::new();

        let error = world.spawn_columns(&mut columns).unwrap_err();

        assert_eq!(error.column_index(), 1);
        assert_eq!(error.expected(), 2);
        assert_eq!(error.actual(), 1);
        assert_eq!(columns.0.len(), 2);
        assert_eq!(columns.1.len(), 1);
        assert_eq!(world.entity_count(), 0);
        assert_eq!(world.archetype_count(), 0);
    }

    #[test]
    fn transfers_non_copy_and_zero_sized_components_across_chunks() {
        let drops = Arc::new(AtomicUsize::new(0));
        let values: Vec<_> = (0..300)
            .map(|value| DropTracked {
                drops: drops.clone(),
                value,
                _padding: [value as u8; 1_000],
            })
            .collect();
        let positions: Vec<_> = (0..300)
            .map(|value| Position {
                x: value as f32,
                y: value as f32,
            })
            .collect();
        let markers = vec![Marker; 300];
        let mut columns = (values, positions, markers);
        let mut world = World::new();

        world.spawn_columns(&mut columns).unwrap();

        assert!(world.data[0].chunks.len() > 1);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(world.query::<&DropTracked>().count(), 300);
        assert_eq!(world.query::<&Marker>().count(), 300);
        let mut sum = 0usize;
        world
            .query::<&DropTracked>()
            .for_each(|component| sum += component.value);
        assert_eq!(sum, (0..300usize).sum::<usize>());

        drop(world);
        assert_eq!(drops.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn supports_one_column_and_empty_input() {
        let mut ticks = (vec![Health(1.0), Health(2.0), Health(3.0)],);
        let capacity = ticks.0.capacity();
        let mut world = World::new();

        world.spawn_columns(&mut ticks).unwrap();

        assert!(ticks.0.is_empty());
        assert_eq!(ticks.0.capacity(), capacity);
        assert_eq!(world.query::<&Health>().count(), 3);

        let mut empty = (Vec::<Damage>::with_capacity(8),);
        let archetype_count = world.archetype_count();
        world.spawn_columns(&mut empty).unwrap();
        assert_eq!(empty.0.capacity(), 8);
        assert_eq!(world.archetype_count(), archetype_count);
    }

    #[test]
    fn reuses_vacant_records_without_reviving_stale_ids() {
        let mut world = World::new();
        let existing = (0..8)
            .map(|value| {
                world.spawn((
                    Position {
                        x: value as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: 0.0 },
                    Health(10.0),
                    Damage(1.0),
                ))
            })
            .collect::<Vec<_>>();
        let stale = existing[..4].to_vec();
        for entity in &stale {
            assert!(world.despawn(*entity));
        }
        let mut columns = (
            vec![Position { x: 2.0, y: 3.0 }; 10],
            vec![Velocity { x: 4.0, y: 5.0 }; 10],
            vec![Health(6.0); 10],
            vec![Damage(7.0); 10],
        );

        world.spawn_columns(&mut columns).unwrap();

        assert_eq!(world.entity_count(), 14);
        assert_eq!(world.entities.len(), 14);
        assert_eq!(
            world
                .query::<(&Position, &Velocity, &Health, &Damage)>()
                .count(),
            14
        );
        assert!(stale.into_iter().all(|entity| !world.contains(entity)));
    }

    #[test]
    fn supports_sixteen_component_columns() {
        macro_rules! define_components {
            ($($name:ident),+ $(,)?) => {
                $(
                    #[derive(Clone, Copy)]
                    #[allow(dead_code)]
                    struct $name(u8);
                )+
            };
        }

        define_components!(C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15);

        let mut columns = (
            vec![C0(0); 4],
            vec![C1(1); 4],
            vec![C2(2); 4],
            vec![C3(3); 4],
            vec![C4(4); 4],
            vec![C5(5); 4],
            vec![C6(6); 4],
            vec![C7(7); 4],
            vec![C8(8); 4],
            vec![C9(9); 4],
            vec![C10(10); 4],
            vec![C11(11); 4],
            vec![C12(12); 4],
            vec![C13(13); 4],
            vec![C14(14); 4],
            vec![C15(15); 4],
        );
        let mut world = World::new();

        world.spawn_columns(&mut columns).unwrap();

        assert_eq!(world.query::<&C0>().count(), 4);
        assert_eq!(world.query::<&C15>().count(), 4);
        world
            .query::<&C0>()
            .for_each(|value| assert_eq!(value.0, 0));
        world
            .query::<&C15>()
            .for_each(|value| assert_eq!(value.0, 15));
    }
}

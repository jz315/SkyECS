use super::{CommandBuffer, World};

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

#[test]
fn storage_epoch_tracks_layout_changes_but_not_value_updates() {
    let mut world = World::new();
    assert_eq!(world.storage_epoch(), 0);

    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
    assert_eq!(world.storage_epoch(), 1);

    world.get_mut::<Position>(entity).unwrap().x = 3.0;
    assert!(world.insert(entity, Position { x: 4.0, y: 5.0 }));
    assert_eq!(world.storage_epoch(), 1);

    world.spawn_batch([
        (Position { x: 6.0, y: 7.0 },),
        (Position { x: 8.0, y: 9.0 },),
    ]);
    assert_eq!(world.storage_epoch(), 2);

    assert!(world.insert(entity, Velocity { x: 1.0, y: 1.0 }));
    assert_eq!(world.storage_epoch(), 3);
    assert!(world.remove::<Velocity>(entity));
    assert_eq!(world.storage_epoch(), 4);

    assert!(world.despawn(entity));
    assert_eq!(world.storage_epoch(), 5);
    assert!(!world.despawn(entity));
    assert_eq!(world.storage_epoch(), 5);

    world.clear();
    assert_eq!(world.storage_epoch(), 6);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Tick(u64);

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct HugeState([u8; 16 * 1024]);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Health(f32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Damage(f32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker;

#[test]
fn chunk_routes_follow_swap_remove_and_reuse_retired_ids() {
    let mut world = World::new();
    let first = world.spawn((HugeState([1; 16 * 1024]), Tick(1)));
    let first_route = world.entity_route(first).unwrap();

    let moved = loop {
        let entity = world.spawn((HugeState([2; 16 * 1024]), Tick(2)));
        if world.data[0].chunks.len() == 2 {
            break entity;
        }
    };
    let retired_route = world.entity_route(moved).unwrap();
    assert_ne!(retired_route.chunk_id, first_route.chunk_id);

    assert!(world.despawn(first));

    let moved_route = world.entity_route(moved).unwrap();
    assert_eq!(moved_route.chunk_id, first_route.chunk_id);
    assert_eq!(world.chunk_directory.resolve(retired_route.chunk_id), None);
    assert_eq!(world.accessor::<Tick>().get(moved), Some(&Tick(2)));

    let replacement = world.spawn((Velocity { x: 3.0, y: 4.0 },));
    assert_eq!(
        world.entity_route(replacement).unwrap().chunk_id,
        retired_route.chunk_id
    );
}

#[test]
fn spawn_initializes_components_and_get_reads_them() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 }, Velocity { x: 3.0, y: 4.0 }));

    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { x: 3.0, y: 4.0 })
    );
}

#[test]
fn spawn_zero_sized_marker_component() {
    let mut world = World::new();
    let entity = world.spawn((Marker,));

    assert!(world.contains(entity));
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.get::<Marker>(entity), Some(&Marker));
}

#[test]
fn spawn_batch_initializes_all_component_columns() {
    let mut world = World::new();
    let expected: Vec<_> = (0..128)
        .map(|i| {
            (
                Position {
                    x: i as f32,
                    y: i as f32 + 0.5,
                },
                Velocity {
                    x: i as f32 * 2.0,
                    y: i as f32 * 2.0 + 1.0,
                },
            )
        })
        .collect();

    world.spawn_batch(expected.iter().copied());

    let mut actual = Vec::new();
    let query = world.query::<(&Position, &Velocity)>();
    query.for_each(|(position, velocity)| {
        actual.push((*position, *velocity));
    });
    actual.sort_by(|(left, _), (right, _)| left.x.total_cmp(&right.x));

    assert_eq!(actual, expected);
}

#[test]
fn insert_preserves_components_when_source_chunk_uses_large_layout() {
    let mut world = World::new();
    let mut migrated = None;

    for i in 0..4097 {
        let entity = world.spawn((
            Position {
                x: i as f32,
                y: i as f32 + 0.5,
            },
            Velocity {
                x: i as f32 * 2.0,
                y: i as f32 * 3.0,
            },
        ));
        if i == 4096 {
            migrated = Some(entity);
        }
    }

    let entity = migrated.unwrap();
    assert!(world.insert(entity, Health(7.0)));
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position {
            x: 4096.0,
            y: 4096.5
        })
    );
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity {
            x: 8192.0,
            y: 12288.0
        })
    );
    assert_eq!(world.get::<Health>(entity), Some(&Health(7.0)));
}

#[test]
fn spawn_batch_empty_iterator_is_noop() {
    let mut world = World::new();

    world.spawn_batch(Vec::<(Position,)>::new());

    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.archetype_count(), 0);
    let positions = world.query::<&Position>();
    assert!(positions.is_empty());
    assert_eq!(positions.cached_archetype_count(), 0);
}

#[test]
fn spawn_batch_reuses_cleared_entity_records_without_growing_capacity() {
    let mut world = World::new();
    world.spawn_batch((0..64).map(|index| (Tick(index),)));
    while world.entities.len() < world.entities.capacity() {
        world.spawn((Tick(world.entities.len() as u64),));
    }
    let record_count = world.entities.len();
    let capacity = world.entities.capacity();

    world.clear();
    world.spawn_batch((0..record_count).map(|index| (Tick(index as u64),)));

    assert_eq!(world.entities.len(), record_count);
    assert_eq!(world.entities.capacity(), capacity);
    assert!(world.free_entities.is_empty());
}

#[test]
fn spawn_batch_reuses_despawned_entity_records_without_growing_capacity() {
    let mut world = World::new();
    let mut entities = (0..64)
        .map(|index| world.spawn((Tick(index),)))
        .collect::<Vec<_>>();
    while world.entities.len() < world.entities.capacity() {
        entities.push(world.spawn((Tick(world.entities.len() as u64),)));
    }
    let record_count = world.entities.len();
    let capacity = world.entities.capacity();
    for entity in entities {
        assert!(world.despawn(entity));
    }

    world.spawn_batch((0..record_count).map(|index| (Tick(index as u64),)));

    assert_eq!(world.entities.len(), record_count);
    assert_eq!(world.entities.capacity(), capacity);
    assert!(world.free_entities.is_empty());
}

#[test]
fn spawn_batch_reserves_only_records_not_covered_by_free_slots() {
    let mut world = World::new();
    let mut entities = (0..64)
        .map(|index| world.spawn((Tick(index),)))
        .collect::<Vec<_>>();
    while world.entities.len() < world.entities.capacity() {
        entities.push(world.spawn((Tick(world.entities.len() as u64),)));
    }
    let record_count = world.entities.len();
    let reused_records = record_count / 2;
    for entity in entities.into_iter().take(reused_records) {
        assert!(world.despawn(entity));
    }

    world.entities.reserve_exact(record_count);
    let capacity = world.entities.capacity();
    let additional_records = capacity - record_count;
    assert!(additional_records > 0);
    let batch_size = reused_records + additional_records;

    world.spawn_batch((0..batch_size).map(|index| (Tick(index as u64),)));

    assert_eq!(world.entities.len(), capacity);
    assert_eq!(world.entities.capacity(), capacity);
    assert!(world.free_entities.is_empty());
}

#[test]
fn get_mut_updates_entity_component() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));

    let position = world.get_mut::<Position>(entity).unwrap();
    position.x = 5.0;

    assert_eq!(world.get::<Position>(entity).unwrap().x, 5.0);
}

#[test]
fn despawn_invalidates_entity_and_keeps_other_entities_accessible() {
    let mut world = World::new();
    let first = world.spawn((Position { x: 1.0, y: 2.0 },));
    let second = world.spawn((Position { x: 3.0, y: 4.0 },));

    assert!(world.despawn(first));
    assert!(!world.contains(first));
    assert_eq!(world.get::<Position>(first), None);
    assert_eq!(
        world.get::<Position>(second),
        Some(&Position { x: 3.0, y: 4.0 })
    );
}

#[test]
fn exhausted_generation_retires_entity_slot_permanently() {
    let mut world = World::new();
    let original = world.spawn((Position { x: 1.0, y: 2.0 },));
    let index = original.index();
    world.entities[index as usize].generation = u32::MAX;
    let final_generation = crate::ecs::EntityId::new(index, u32::MAX);

    assert!(world.despawn(final_generation));
    assert_eq!(world.entity_count(), 0);
    assert!(!world.free_entities.contains(&index));

    let replacement = world.spawn((Position { x: 3.0, y: 4.0 },));
    assert_ne!(replacement.index(), index);
    assert!(!world.contains(final_generation));
}

#[test]
fn clear_does_not_recycle_an_exhausted_generation() {
    let mut world = World::new();
    let retired = world.spawn((Position { x: 1.0, y: 2.0 },));
    let reusable = world.spawn((Position { x: 3.0, y: 4.0 },));
    world.entities[retired.index() as usize].generation = u32::MAX;

    world.clear();
    let replacement = world.spawn((Position { x: 5.0, y: 6.0 },));

    assert_eq!(replacement.index(), reusable.index());
    assert_ne!(replacement.index(), retired.index());
    assert!(!world.contains(crate::ecs::EntityId::new(retired.index(), u32::MAX)));
}

#[test]
fn insert_adds_new_component_and_keeps_existing_values() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));

    assert!(world.insert(entity, Velocity { x: 3.0, y: 4.0 }));
    assert!(world.has::<Velocity>(entity));
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { x: 3.0, y: 4.0 })
    );
}

#[test]
fn remove_moves_entity_to_smaller_archetype() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 }, Velocity { x: 3.0, y: 4.0 }));

    assert!(world.remove::<Velocity>(entity));
    assert!(!world.has::<Velocity>(entity));
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(world.get::<Velocity>(entity), None);
}

#[test]
fn remove_can_leave_entity_with_only_zero_sized_component() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 }, Marker));

    assert!(world.remove::<Position>(entity));
    assert!(world.contains(entity));
    assert!(!world.has::<Position>(entity));
    assert!(world.has::<Marker>(entity));
    assert_eq!(world.get::<Marker>(entity), Some(&Marker));
}

#[test]
fn remove_last_component_leaves_empty_entity_alive() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));

    assert!(world.remove::<Position>(entity));
    assert!(world.contains(entity));
    assert!(!world.has::<Position>(entity));
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn resources_can_be_inserted_read_mutated_and_removed() {
    let mut world = World::new();

    assert!(!world.contains_resource::<Tick>());
    assert_eq!(world.insert_resource(Tick(1)), None);
    assert!(world.contains_resource::<Tick>());
    assert_eq!(world.get_resource::<Tick>(), Some(&Tick(1)));

    world.get_resource_mut::<Tick>().unwrap().0 += 1;
    assert_eq!(world.get_resource::<Tick>(), Some(&Tick(2)));
    assert_eq!(world.remove_resource::<Tick>(), Some(Tick(2)));
    assert!(!world.contains_resource::<Tick>());
}

#[test]
fn commands_apply_spawns_components_and_resources() {
    let mut world = World::new();
    let mut commands = CommandBuffer::new();

    commands.insert_resource(Tick(10));
    commands.spawn((Position { x: 7.0, y: 8.0 }, Velocity { x: 1.0, y: 2.0 }));
    commands.apply(&mut world);

    assert_eq!(world.get_resource::<Tick>(), Some(&Tick(10)));

    let mut count = 0usize;
    let query = world.query::<(&Position, &Velocity)>();
    query.for_each(|(position, velocity)| {
        count += 1;
        assert_eq!(position, &Position { x: 7.0, y: 8.0 });
        assert_eq!(velocity, &Velocity { x: 1.0, y: 2.0 });
    });

    assert_eq!(count, 1);
}

#[test]
fn commands_apply_entity_changes_in_order() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
    let mut commands = CommandBuffer::new();

    commands.insert(entity, Velocity { x: 3.0, y: 4.0 });
    commands.remove::<Velocity>(entity);
    commands.despawn(entity);
    commands.apply(&mut world);

    assert!(!world.contains(entity));
}

#[test]
fn commands_coalesce_repeated_component_changes_per_entity() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
    let mut commands = CommandBuffer::new();

    commands.insert(entity, Velocity { x: 1.0, y: 1.0 });
    commands.insert(entity, Velocity { x: 5.0, y: 8.0 });
    commands.remove::<Velocity>(entity);
    commands.insert(entity, Velocity { x: 13.0, y: 21.0 });
    commands.apply(&mut world);

    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { x: 13.0, y: 21.0 })
    );
}

#[test]
fn commands_ignore_component_changes_after_despawn() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
    let mut commands = CommandBuffer::new();

    commands.insert(entity, Velocity { x: 1.0, y: 1.0 });
    commands.despawn(entity);
    commands.insert(entity, Velocity { x: 5.0, y: 8.0 });
    commands.remove::<Position>(entity);
    commands.apply(&mut world);

    assert!(!world.contains(entity));
    assert_eq!(world.get::<Velocity>(entity), None);
    assert_eq!(world.get::<Position>(entity), None);
}

#[test]
fn commands_keep_distinct_generations_with_same_entity_index_separate() {
    let mut world = World::new();
    let stale = world.spawn((Position { x: 1.0, y: 2.0 },));
    assert!(world.despawn(stale));
    let current = world.spawn((Position { x: 3.0, y: 4.0 },));
    assert_eq!(stale.index(), current.index());
    assert_ne!(stale.generation(), current.generation());

    let mut commands = CommandBuffer::new();
    commands.insert(stale, Velocity { x: 1.0, y: 1.0 });
    commands.insert(current, Velocity { x: 5.0, y: 8.0 });
    commands.apply(&mut world);

    assert_eq!(world.get::<Velocity>(stale), None);
    assert_eq!(
        world.get::<Velocity>(current),
        Some(&Velocity { x: 5.0, y: 8.0 })
    );
}

#[test]
fn commands_batch_add_component_spans_multiple_chunks() {
    let mut world = World::new();
    let entities: Vec<_> = (0..40)
        .map(|i| {
            world.spawn((
                HugeState([i as u8; 16 * 1024]),
                Position {
                    x: i as f32,
                    y: i as f32 + 0.5,
                },
            ))
        })
        .collect();

    let mut commands = CommandBuffer::new();
    for &entity in &entities {
        commands.insert(entity, Velocity { x: 3.0, y: 4.0 });
    }
    commands.apply(&mut world);

    for (i, &entity) in entities.iter().enumerate() {
        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: i as f32,
                y: i as f32 + 0.5,
            })
        );
        assert_eq!(
            world.get::<Velocity>(entity),
            Some(&Velocity { x: 3.0, y: 4.0 })
        );
    }
}

#[test]
fn commands_batch_remove_component_keeps_moved_survivor_locations_valid() {
    let mut world = World::new();
    let entities: Vec<_> = (0..40)
        .map(|i| {
            world.spawn((
                HugeState([i as u8; 16 * 1024]),
                Position {
                    x: i as f32,
                    y: i as f32 + 0.5,
                },
                Velocity {
                    x: i as f32 * 10.0,
                    y: i as f32 * 10.0 + 1.0,
                },
            ))
        })
        .collect();

    let mut commands = CommandBuffer::new();
    for &entity in &entities[..24] {
        commands.remove::<Velocity>(entity);
    }
    commands.apply(&mut world);

    for (i, &entity) in entities.iter().enumerate() {
        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: i as f32,
                y: i as f32 + 0.5,
            })
        );

        if i < 24 {
            assert_eq!(world.get::<Velocity>(entity), None);
        } else {
            assert_eq!(
                world.get::<Velocity>(entity),
                Some(&Velocity {
                    x: i as f32 * 10.0,
                    y: i as f32 * 10.0 + 1.0,
                })
            );
        }
    }
}

#[test]
fn commands_generic_batch_add_component_handles_multiple_targets_from_same_source() {
    let mut world = World::new();
    let entities: Vec<_> = (0..24)
        .map(|i| {
            world.spawn((
                Position {
                    x: i as f32,
                    y: i as f32 + 0.5,
                },
                Velocity {
                    x: i as f32 * 2.0,
                    y: i as f32 * 2.0 + 1.0,
                },
            ))
        })
        .collect();

    let mut commands = CommandBuffer::new();
    for (i, &entity) in entities.iter().enumerate() {
        if i % 2 == 0 {
            commands.insert(entity, Health(100.0 + i as f32));
        } else {
            commands.insert(entity, Damage(i as f32));
        }
    }
    commands.apply(&mut world);

    for (i, &entity) in entities.iter().enumerate() {
        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position {
                x: i as f32,
                y: i as f32 + 0.5,
            })
        );
        assert_eq!(
            world.get::<Velocity>(entity),
            Some(&Velocity {
                x: i as f32 * 2.0,
                y: i as f32 * 2.0 + 1.0,
            })
        );

        if i % 2 == 0 {
            assert_eq!(world.get::<Health>(entity), Some(&Health(100.0 + i as f32)));
            assert_eq!(world.get::<Damage>(entity), None);
        } else {
            assert_eq!(world.get::<Health>(entity), None);
            assert_eq!(world.get::<Damage>(entity), Some(&Damage(i as f32)));
        }
    }
}

#[test]
fn entity_count_tracks_live_entities() {
    let mut world = World::new();
    assert_eq!(world.entity_count(), 0);

    let a = world.spawn((Position { x: 0.0, y: 0.0 },));
    let b = world.spawn((Position { x: 1.0, y: 1.0 },));
    assert_eq!(world.entity_count(), 2);

    world.despawn(a);
    assert_eq!(world.entity_count(), 1);

    world.despawn(b);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn archetype_count_grows_with_distinct_shapes() {
    let mut world = World::new();
    assert_eq!(world.archetype_count(), 0);

    world.spawn((Position { x: 0.0, y: 0.0 },));
    assert_eq!(world.archetype_count(), 1);

    world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 1.0 }));
    assert_eq!(world.archetype_count(), 2);

    // same shape reuses existing archetype
    world.spawn((Position { x: 2.0, y: 2.0 },));
    assert_eq!(world.archetype_count(), 2);
}

#[test]
fn component_postings_match_world_archetype_columns() {
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));
    assert!(world.insert(entity, Velocity { x: 3.0, y: 4.0 }));
    world.spawn((Velocity { x: 5.0, y: 6.0 },));

    for (data_index, data) in world.data.iter().enumerate() {
        for (column_index, component) in data.archetype.components.iter().enumerate() {
            let list = world.component_postings.list(component).unwrap();
            let posting_index = list.first_at_or_after(data_index);
            let entry = list.entry(posting_index).unwrap();
            assert_eq!(entry.data_index(), data_index);
            assert_eq!(entry.column_index() as usize, column_index);
        }
    }
}

#[test]
fn clear_removes_all_entities_but_keeps_resources() {
    let mut world = World::new();
    world.insert_resource(Tick(42));
    let entity = world.spawn((Position { x: 1.0, y: 2.0 },));

    world.clear();

    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.archetype_count(), 0);
    assert!(!world.contains(entity));
    assert_eq!(world.get_resource::<Tick>(), Some(&Tick(42)));
}

#[test]
fn clear_invalidates_entity_ids_before_reuse() {
    let mut world = World::new();
    let stale = world.spawn((Position { x: 1.0, y: 2.0 },));

    world.clear();
    let current = world.spawn((Position { x: 3.0, y: 4.0 },));

    assert_eq!(stale.index(), current.index());
    assert_ne!(stale.generation(), current.generation());
    assert!(!world.contains(stale));
    assert_eq!(world.get::<Position>(stale), None);
    assert_eq!(
        world.get::<Position>(current),
        Some(&Position { x: 3.0, y: 4.0 })
    );
}

#[test]
fn commands_flush_in_first_seen_entity_order() {
    // Entity A is seen first; even though a later command targets A again,
    // all of A's coalesced commands should execute before B's.
    let mut world = World::new();
    let a = world.spawn((Position { x: 1.0, y: 2.0 },));
    let b = world.spawn((Position { x: 3.0, y: 4.0 },));

    let mut cmds = CommandBuffer::new();
    cmds.insert(a, Health(10.0)); // A first seen
    cmds.insert(b, Health(20.0)); // B first seen
    cmds.remove::<Health>(a); // A again — coalesces with first A entry
    cmds.apply(&mut world);

    // A: insert Health then remove Health → net result: no Health
    assert_eq!(world.get::<Health>(a), None);
    // B: insert Health → Health present
    assert_eq!(world.get::<Health>(b), Some(&Health(20.0)));
}

// =======================================================================
// Drop semantics tests
// =======================================================================

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// A component whose Drop increments a shared counter.
#[derive(Clone)]
struct Droppable {
    counter: Arc<AtomicUsize>,
}

impl Droppable {
    fn new(counter: &Arc<AtomicUsize>) -> Self {
        Self {
            counter: counter.clone(),
        }
    }
}

impl Drop for Droppable {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn despawn_calls_drop_on_components() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Droppable::new(&counter),));
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    world.despawn(entity);
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

/// A second droppable type to test multi-component bundles.
#[derive(Clone)]
struct DroppableB {
    counter: Arc<AtomicUsize>,
}

impl DroppableB {
    fn new(counter: &Arc<AtomicUsize>) -> Self {
        Self {
            counter: counter.clone(),
        }
    }
}

impl Drop for DroppableB {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
}

struct PanicDropA {
    should_panic: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
}

impl Drop for PanicDropA {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
        if self.should_panic.swap(false, Ordering::Relaxed) {
            panic!("PanicDropA");
        }
    }
}

struct PanicDropB {
    should_panic: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
}

impl Drop for PanicDropB {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
        if self.should_panic.swap(false, Ordering::Relaxed) {
            panic!("PanicDropB");
        }
    }
}

struct OwnedPanicDropA {
    label: String,
    bytes: Box<[u8]>,
    should_panic: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
}

impl OwnedPanicDropA {
    fn new(label: &str, should_panic: Arc<AtomicBool>, drops: Arc<AtomicUsize>) -> Self {
        Self {
            label: label.to_owned(),
            bytes: vec![label.len() as u8; 32].into_boxed_slice(),
            should_panic,
            drops,
        }
    }
}

impl Drop for OwnedPanicDropA {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
        if self.should_panic.swap(false, Ordering::Relaxed) {
            panic!("OwnedPanicDropA({})", self.label);
        }
    }
}

struct OwnedPanicDropB {
    values: Vec<String>,
    should_panic: Arc<AtomicBool>,
    drops: Arc<AtomicUsize>,
}

impl OwnedPanicDropB {
    fn new(value: &str, should_panic: Arc<AtomicBool>, drops: Arc<AtomicUsize>) -> Self {
        Self {
            values: vec![value.to_owned(), format!("{value}-owned")],
            should_panic,
            drops,
        }
    }
}

impl Drop for OwnedPanicDropB {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
        if self.should_panic.swap(false, Ordering::Relaxed) {
            panic!("OwnedPanicDropB({})", self.values[0]);
        }
    }
}

#[test]
fn despawn_calls_drop_on_multiple_components() {
    let counter_a = Arc::new(AtomicUsize::new(0));
    let counter_b = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Droppable::new(&counter_a), DroppableB::new(&counter_b)));

    world.despawn(entity);
    assert_eq!(counter_a.load(Ordering::Relaxed), 1);
    assert_eq!(counter_b.load(Ordering::Relaxed), 1);
}

#[test]
fn direct_despawn_finishes_before_component_drop_panic_resumes() {
    let removed_a_drops = Arc::new(AtomicUsize::new(0));
    let removed_b_drops = Arc::new(AtomicUsize::new(0));
    let survivor_a_drops = Arc::new(AtomicUsize::new(0));
    let survivor_b_drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let removed = world.spawn((
        Position { x: 1.0, y: 2.0 },
        OwnedPanicDropA::new(
            "removed-a",
            Arc::new(AtomicBool::new(true)),
            removed_a_drops.clone(),
        ),
        OwnedPanicDropB::new(
            "removed-b",
            Arc::new(AtomicBool::new(true)),
            removed_b_drops.clone(),
        ),
    ));
    let survivor = world.spawn((
        Position { x: 3.0, y: 4.0 },
        OwnedPanicDropA::new(
            "survivor-a",
            Arc::new(AtomicBool::new(false)),
            survivor_a_drops.clone(),
        ),
        OwnedPanicDropB::new(
            "survivor-b",
            Arc::new(AtomicBool::new(false)),
            survivor_b_drops.clone(),
        ),
    ));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.despawn(removed);
    }));

    assert!(result.is_err());
    assert!(!world.contains(removed));
    assert_eq!(world.entity_count(), 1);
    assert_eq!(
        world.get::<Position>(survivor),
        Some(&Position { x: 3.0, y: 4.0 })
    );
    assert_eq!(
        world.get::<OwnedPanicDropA>(survivor).unwrap().label,
        "survivor-a"
    );
    assert_eq!(removed_a_drops.load(Ordering::Relaxed), 1);
    assert_eq!(removed_b_drops.load(Ordering::Relaxed), 1);
    assert_eq!(survivor_a_drops.load(Ordering::Relaxed), 0);
    assert_eq!(survivor_b_drops.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(removed_a_drops.load(Ordering::Relaxed), 1);
    assert_eq!(removed_b_drops.load(Ordering::Relaxed), 1);
    assert_eq!(survivor_a_drops.load(Ordering::Relaxed), 1);
    assert_eq!(survivor_b_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn direct_insert_installs_replacement_before_old_drop_panic_resumes() {
    let old_drops = Arc::new(AtomicUsize::new(0));
    let replacement_drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((
        Position { x: 1.0, y: 2.0 },
        OwnedPanicDropA::new("old", Arc::new(AtomicBool::new(true)), old_drops.clone()),
    ));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.insert(
            entity,
            OwnedPanicDropA::new(
                "replacement",
                Arc::new(AtomicBool::new(false)),
                replacement_drops.clone(),
            ),
        );
    }));

    assert!(result.is_err());
    assert!(world.contains(entity));
    let replacement = world.get::<OwnedPanicDropA>(entity).unwrap();
    assert_eq!(replacement.label, "replacement");
    assert_eq!(replacement.bytes.len(), 32);
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(replacement_drops.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(replacement_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn direct_remove_finishes_migration_before_drop_panic_resumes() {
    let removed_drops = Arc::new(AtomicUsize::new(0));
    let survivor_drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let removed = world.spawn((
        Position { x: 1.0, y: 2.0 },
        OwnedPanicDropA::new(
            "removed",
            Arc::new(AtomicBool::new(true)),
            removed_drops.clone(),
        ),
    ));
    let survivor = world.spawn((
        Position { x: 3.0, y: 4.0 },
        OwnedPanicDropA::new(
            "survivor",
            Arc::new(AtomicBool::new(false)),
            survivor_drops.clone(),
        ),
    ));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.remove::<OwnedPanicDropA>(removed);
    }));

    assert!(result.is_err());
    assert!(world.contains(removed));
    assert!(world.get::<OwnedPanicDropA>(removed).is_none());
    assert_eq!(
        world.get::<Position>(removed),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        world.get::<OwnedPanicDropA>(survivor).unwrap().label,
        "survivor"
    );
    assert_eq!(
        world.get::<Position>(survivor),
        Some(&Position { x: 3.0, y: 4.0 })
    );
    assert_eq!(world.entity_count(), 2);
    assert_eq!(removed_drops.load(Ordering::Relaxed), 1);
    assert_eq!(survivor_drops.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(removed_drops.load(Ordering::Relaxed), 1);
    assert_eq!(survivor_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn remove_component_calls_drop_on_removed_column() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Position { x: 0.0, y: 0.0 }, Droppable::new(&counter)));

    world.remove::<Droppable>(entity);
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    // Entity should still be alive with its Position.
    assert!(world.contains(entity));
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 0.0, y: 0.0 })
    );
}

#[test]
fn insert_overwrite_calls_drop_on_old_value() {
    let counter_old = Arc::new(AtomicUsize::new(0));
    let counter_new = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Droppable::new(&counter_old),));

    world.insert(entity, Droppable::new(&counter_new));
    // Old value should have been dropped.
    assert_eq!(counter_old.load(Ordering::Relaxed), 1);
    // New value should not have been dropped yet.
    assert_eq!(counter_new.load(Ordering::Relaxed), 0);
}

#[test]
fn commands_apply_final_archetype_with_one_storage_migration() {
    let mut world = World::new();
    let entities = (0..8)
        .map(|index| {
            world.spawn((Position {
                x: index as f32,
                y: index as f32 + 0.5,
            },))
        })
        .collect::<Vec<_>>();
    let epoch_before = world.storage_epoch;

    let mut commands = CommandBuffer::new();
    for (index, &entity) in entities.iter().enumerate() {
        commands.insert(
            entity,
            Velocity {
                x: index as f32 * 2.0,
                y: index as f32 * 3.0,
            },
        );
        commands.insert(entity, Health(index as f32 + 10.0));
        commands.remove::<Position>(entity);
    }
    commands.apply(&mut world);

    assert_eq!(world.storage_epoch, epoch_before + entities.len() as u64);
    assert_eq!(
        world.data.len(),
        2,
        "only source and final storage are built"
    );
    assert_eq!(world.component_command_transitions.len(), 1);
    for (index, &entity) in entities.iter().enumerate() {
        assert_eq!(world.get::<Position>(entity), None);
        assert_eq!(
            world.get::<Velocity>(entity),
            Some(&Velocity {
                x: index as f32 * 2.0,
                y: index as f32 * 3.0,
            })
        );
        assert_eq!(
            world.get::<Health>(entity),
            Some(&Health(index as f32 + 10.0))
        );
    }
}

#[test]
fn single_component_commands_keep_the_cached_transition_fast_path() {
    let mut world = World::new();
    let entities = (0..32)
        .map(|index| {
            world.spawn((Position {
                x: index as f32,
                y: index as f32,
            },))
        })
        .collect::<Vec<_>>();
    let epoch_before = world.storage_epoch;
    let mut commands = CommandBuffer::new();
    for &entity in &entities {
        commands.insert(entity, Velocity { x: 1.0, y: 2.0 });
    }

    commands.apply(&mut world);

    assert_eq!(world.storage_epoch, epoch_before + entities.len() as u64);
    assert_eq!(world.transitions.len(), 1);
    assert!(world.component_command_transitions.is_empty());
    assert_eq!(world.data.len(), 2);
    for entity in entities {
        assert_eq!(
            world.get::<Velocity>(entity),
            Some(&Velocity { x: 1.0, y: 2.0 })
        );
    }
}

#[test]
fn commands_single_migration_preserves_overwrite_and_drop_ownership() {
    let old = Arc::new(AtomicUsize::new(0));
    let superseded = Arc::new(AtomicUsize::new(0));
    let final_value = Arc::new(AtomicUsize::new(0));
    let removed = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((
        Position { x: 1.0, y: 2.0 },
        Droppable::new(&old),
        DroppableB::new(&removed),
    ));
    let epoch_before = world.storage_epoch;

    let mut commands = CommandBuffer::new();
    commands.insert(entity, Droppable::new(&superseded));
    commands.insert(entity, Droppable::new(&final_value));
    assert_eq!(superseded.load(Ordering::Relaxed), 1);
    commands.remove::<DroppableB>(entity);
    commands.insert(entity, Velocity { x: 8.0, y: 13.0 });
    commands.apply(&mut world);

    assert_eq!(world.storage_epoch, epoch_before + 1);
    assert_eq!(old.load(Ordering::Relaxed), 1);
    assert_eq!(removed.load(Ordering::Relaxed), 1);
    assert_eq!(superseded.load(Ordering::Relaxed), 1);
    assert_eq!(final_value.load(Ordering::Relaxed), 0);
    assert!(world.get::<Droppable>(entity).is_some());
    assert!(world.get::<DroppableB>(entity).is_none());
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { x: 8.0, y: 13.0 })
    );

    drop(world);
    assert_eq!(old.load(Ordering::Relaxed), 1);
    assert_eq!(removed.load(Ordering::Relaxed), 1);
    assert_eq!(superseded.load(Ordering::Relaxed), 1);
    assert_eq!(final_value.load(Ordering::Relaxed), 1);
}

#[test]
fn single_component_command_finishes_migration_before_drop_panic_resumes() {
    let should_panic = Arc::new(AtomicBool::new(true));
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((
        Position { x: 1.0, y: 2.0 },
        PanicDropA {
            should_panic,
            drops: drops.clone(),
        },
    ));
    let mut commands = CommandBuffer::new();
    commands.remove::<PanicDropA>(entity);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        commands.apply(&mut world);
    }));

    assert!(result.is_err());
    assert!(commands.is_empty());
    assert!(world.is_poisoned());
    assert!(world.contains(entity));
    assert!(world.get::<PanicDropA>(entity).is_none());
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(drops.load(Ordering::Relaxed), 1);

    // The captured panic resumes only after source compaction, so normal
    // World destruction cannot double-drop the removed value.
    drop(world);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn single_component_overwrite_installs_new_value_before_drop_panic_resumes() {
    let old_should_panic = Arc::new(AtomicBool::new(true));
    let old_drops = Arc::new(AtomicUsize::new(0));
    let new_should_panic = Arc::new(AtomicBool::new(false));
    let new_drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((PanicDropA {
        should_panic: old_should_panic,
        drops: old_drops.clone(),
    },));
    let mut commands = CommandBuffer::new();
    commands.insert(
        entity,
        PanicDropA {
            should_panic: new_should_panic,
            drops: new_drops.clone(),
        },
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        commands.apply(&mut world);
    }));

    assert!(result.is_err());
    assert!(commands.is_empty());
    assert!(world.is_poisoned());
    assert!(world.get::<PanicDropA>(entity).is_some());
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(new_drops.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(old_drops.load(Ordering::Relaxed), 1);
    assert_eq!(new_drops.load(Ordering::Relaxed), 1);
}

#[test]
fn multi_component_command_retains_first_drop_panic_and_finishes_migration() {
    let panic_a = Arc::new(AtomicBool::new(true));
    let panic_b = Arc::new(AtomicBool::new(true));
    let drops_a = Arc::new(AtomicUsize::new(0));
    let drops_b = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((
        Position { x: 1.0, y: 2.0 },
        PanicDropA {
            should_panic: panic_a,
            drops: drops_a.clone(),
        },
        PanicDropB {
            should_panic: panic_b,
            drops: drops_b.clone(),
        },
    ));
    let mut commands = CommandBuffer::new();
    commands.remove::<PanicDropA>(entity);
    commands.remove::<PanicDropB>(entity);
    commands.insert(entity, Velocity { x: 3.0, y: 4.0 });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        commands.apply(&mut world);
    }));

    assert!(result.is_err());
    assert!(commands.is_empty());
    assert!(world.is_poisoned());
    assert!(world.contains(entity));
    assert!(world.get::<PanicDropA>(entity).is_none());
    assert!(world.get::<PanicDropB>(entity).is_none());
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        world.get::<Velocity>(entity),
        Some(&Velocity { x: 3.0, y: 4.0 })
    );
    assert_eq!(drops_a.load(Ordering::Relaxed), 1);
    assert_eq!(drops_b.load(Ordering::Relaxed), 1);

    // Both panicking values were discarded exactly once; the second panic
    // did not interrupt ownership repair or cause a double unwind.
    drop(world);
    assert_eq!(drops_a.load(Ordering::Relaxed), 1);
    assert_eq!(drops_b.load(Ordering::Relaxed), 1);
}

#[test]
fn clear_calls_drop_on_all_entity_components() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    for _ in 0..10 {
        world.spawn((Droppable::new(&counter),));
    }
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    world.clear();
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[test]
fn direct_clear_finishes_before_component_drop_panic_resumes() {
    let panic_a = Arc::new(AtomicBool::new(true));
    let panic_b = Arc::new(AtomicBool::new(true));
    let drops_a = Arc::new(AtomicUsize::new(0));
    let drops_b = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    assert!(world
        .insert_resource(String::from("resource survives clear"))
        .is_none());
    let entities = (0..3)
        .map(|index| {
            world.spawn((
                Position {
                    x: index as f32,
                    y: 0.0,
                },
                OwnedPanicDropA::new(&format!("a-{index}"), panic_a.clone(), drops_a.clone()),
                OwnedPanicDropB::new(&format!("b-{index}"), panic_b.clone(), drops_b.clone()),
            ))
        })
        .collect::<Vec<_>>();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.clear();
    }));

    assert!(result.is_err());
    assert_eq!(drops_a.load(Ordering::Relaxed), 3);
    assert_eq!(drops_b.load(Ordering::Relaxed), 3);
    assert_eq!(world.entity_count(), 0);
    assert_eq!(world.archetype_count(), 0);
    assert!(entities.iter().all(|&entity| !world.contains(entity)));
    assert_eq!(
        world.get_resource::<String>().map(String::as_str),
        Some("resource survives clear")
    );

    // The World remains usable after the caller catches the user panic,
    // and recycled slots cannot make any stale EntityId valid again.
    let replacement = world.spawn((Position { x: 9.0, y: 10.0 },));
    assert!(world.contains(replacement));
    assert!(entities.iter().all(|&entity| !world.contains(entity)));

    drop(world);
    assert_eq!(drops_a.load(Ordering::Relaxed), 3);
    assert_eq!(drops_b.load(Ordering::Relaxed), 3);
}

#[test]
fn world_drop_calls_drop_on_all_entity_components() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        for _ in 0..5 {
            world.spawn((Droppable::new(&counter),));
        }
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
    // World was dropped — all components should be dropped.
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn spawn_with_non_copy_component_and_read_back() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Droppable::new(&counter),));

    // We can get an immutable reference to the non-Copy component.
    let droppable = world.get::<Droppable>(entity).unwrap();
    assert!(Arc::ptr_eq(&droppable.counter, &counter));
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

struct GrowthDroppable {
    counter: Arc<AtomicUsize>,
    value: usize,
    _padding: [u8; 1000],
}

impl Drop for GrowthDroppable {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
}

static GROWTH_ZST_DROPS: AtomicUsize = AtomicUsize::new(0);

struct GrowthDroppableZst;

impl Drop for GrowthDroppableZst {
    fn drop(&mut self) {
        GROWTH_ZST_DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn chunk_growth_preserves_non_copy_components_and_drop_ownership() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entities: Vec<_> = (0..300)
        .map(|value| {
            world.spawn((GrowthDroppable {
                counter: counter.clone(),
                value,
                _padding: [value as u8; 1000],
            },))
        })
        .collect();

    for &index in &[0, 1, 4, 16, 64, 255, 299] {
        assert_eq!(
            world.get::<GrowthDroppable>(entities[index]).unwrap().value,
            index
        );
    }
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    assert!(world.despawn(entities[3]));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    assert_eq!(
        world.get::<GrowthDroppable>(entities[299]).unwrap().value,
        299
    );

    drop(world);
    assert_eq!(counter.load(Ordering::Relaxed), 300);
}

#[test]
fn spawn_batch_preserves_non_copy_components_across_chunks() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    world.spawn_batch((0..300).map(|value| {
        (GrowthDroppable {
            counter: counter.clone(),
            value,
            _padding: [value as u8; 1000],
        },)
    }));

    assert!(world.data[0].chunks.len() > 1);
    let mut count = 0;
    let mut sum = 0;
    world.query::<&GrowthDroppable>().for_each(|component| {
        count += 1;
        sum += component.value;
        assert_eq!(component._padding[0], component.value as u8);
    });
    assert_eq!(count, 300);
    assert_eq!(sum, (0..300).sum::<usize>());
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(counter.load(Ordering::Relaxed), 300);
}

#[test]
fn spawn_batch_commits_completed_rows_when_the_iterator_panics() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let components = (0..32)
            .map(|value| {
                (GrowthDroppable {
                    counter: counter.clone(),
                    value,
                    _padding: [value as u8; 1000],
                },)
            })
            .chain(std::iter::once_with(|| -> (GrowthDroppable,) {
                panic!("batch iterator panic")
            }));
        world.spawn_batch(components);
    }));

    assert!(result.is_err());
    assert_eq!(world.entity_count(), 32);
    assert_eq!(world.query::<&GrowthDroppable>().count(), 32);
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    world.spawn((GrowthDroppable {
        counter: counter.clone(),
        value: 32,
        _padding: [32; 1000],
    },));
    assert_eq!(world.entity_count(), 33);

    drop(world);
    assert_eq!(counter.load(Ordering::Relaxed), 33);
}

#[test]
fn chunk_growth_preserves_droppable_zero_sized_components() {
    GROWTH_ZST_DROPS.store(0, Ordering::Relaxed);
    let mut world = World::new();
    for _ in 0..5_000 {
        world.spawn((GrowthDroppableZst,));
    }

    assert_eq!(world.query::<&GrowthDroppableZst>().count(), 5_000);
    assert_eq!(GROWTH_ZST_DROPS.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(GROWTH_ZST_DROPS.load(Ordering::Relaxed), 5_000);
}

#[test]
fn spawn_batch_preserves_droppable_zero_sized_components() {
    GROWTH_ZST_DROPS.store(0, Ordering::Relaxed);
    let mut world = World::new();
    world.spawn_batch((0..5_000).map(|_| (GrowthDroppableZst,)));

    assert_eq!(world.query::<&GrowthDroppableZst>().count(), 5_000);
    assert_eq!(GROWTH_ZST_DROPS.load(Ordering::Relaxed), 0);

    drop(world);
    assert_eq!(GROWTH_ZST_DROPS.load(Ordering::Relaxed), 5_000);
}

#[test]
fn exact_spawn_batch_starts_from_a_batch_appropriate_size_class() {
    #[derive(Clone, Copy)]
    struct HundredBytes([u8; 100]);

    let mut world = World::new();
    world.spawn_batch((0..10_000).map(|index| (HundredBytes([index as u8; 100]),)));

    assert_eq!(world.data.len(), 1);
    assert_eq!(world.data[0].chunks.len(), 1);
    assert_eq!(world.data[0].chunks[0].block_size(), 1024 * 1024);
    assert_eq!(world.query::<&HundredBytes>().count(), 10_000);
    let mut first = None;
    world.query::<&HundredBytes>().for_each(|value| {
        if first.is_none() {
            first = Some(value.0[0]);
        }
    });
    assert_eq!(first, Some(0));
}

#[test]
fn repeated_known_batches_do_not_regress_to_smaller_chunk_classes() {
    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct HundredBytes([u8; 100]);

    let mut world = World::new();
    for batch in 0..5 {
        world.spawn_batch(
            (0..10_000).map(|index| (HundredBytes([(batch * 10_000 + index) as u8; 100]),)),
        );
    }

    let block_sizes = world.data[0]
        .chunks
        .iter()
        .map(|chunk| chunk.block_size())
        .collect::<Vec<_>>();
    assert_eq!(
        block_sizes,
        vec![
            1024 * 1024,
            1024 * 1024,
            1024 * 1024,
            1024 * 1024,
            4 * 1024 * 1024,
        ]
    );
    assert_eq!(world.query::<&HundredBytes>().count(), 50_000);
}

#[test]
fn exact_large_batch_uses_radix_chunks_and_preserves_values() {
    #[derive(Clone, Copy)]
    struct WideRow {
        index: u32,
        padding: [u8; 96],
    }

    const BATCH_COUNT: usize = 180_003;
    let mut world = World::new();
    world.spawn((WideRow {
        index: 0,
        padding: [0; 96],
    },));
    world.spawn_batch((1..=BATCH_COUNT).map(|index| {
        (WideRow {
            index: index as u32,
            padding: [index as u8; 96],
        },)
    }));

    assert_eq!(world.data.len(), 1);
    assert_eq!(world.data[0].chunks.len(), 3);
    assert_eq!(world.data[0].chunks[0].block_size(), 4 * 1024);
    assert_eq!(world.data[0].chunks[1].block_size(), 16 * 1024 * 1024);
    assert_eq!(world.data[0].chunks[2].block_size(), 4 * 1024 * 1024);

    let mut count = 0usize;
    let mut sum = 0u64;
    world.query::<&WideRow>().for_each(|row| {
        count += 1;
        sum += u64::from(row.index);
        assert_eq!(row.padding[0], row.index as u8);
    });
    assert_eq!(count, BATCH_COUNT + 1);
    assert_eq!(sum, (0..=BATCH_COUNT as u64).sum::<u64>());

    world.spawn((WideRow {
        index: (BATCH_COUNT + 1) as u32,
        padding: [0; 96],
    },));

    assert_eq!(world.data[0].chunks.len(), 3);
    assert_eq!(
        world.data[0].chunks.last().unwrap().block_size(),
        4 * 1024 * 1024
    );
    assert_eq!(world.query::<&WideRow>().count(), BATCH_COUNT + 2);
}

#[test]
fn spawn_batch_remains_safe_when_an_iterator_overstates_its_lower_bound() {
    struct OverstatedHint {
        next: u64,
    }

    impl Iterator for OverstatedHint {
        type Item = (Tick,);

        fn next(&mut self) -> Option<Self::Item> {
            let value = self.next;
            if value == 3 {
                return None;
            }
            self.next += 1;
            Some((Tick(value),))
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (1_000, Some(1_000))
        }
    }

    let mut world = World::new();
    world.spawn_batch(OverstatedHint { next: 0 });

    assert_eq!(world.entity_count(), 3);
    let mut sum = 0;
    world.query::<&Tick>().for_each(|tick| sum += tick.0);
    assert_eq!(sum, 3);
}

#[test]
fn unknown_batch_size_falls_back_to_incremental_growth() {
    let mut world = World::new();
    let bundles = (0..5_000)
        .filter(|_| true)
        .map(|value| (Tick(value as u64), Marker));
    assert_eq!(bundles.size_hint().0, 0);

    world.spawn_batch(bundles);

    assert_eq!(world.query::<(&Tick, &Marker)>().count(), 5_000);
    let mut sum = 0u64;
    world
        .query::<&Tick>()
        .for_each(|tick| sum = sum.wrapping_add(tick.0));
    assert_eq!(sum, (0..5_000u64).sum::<u64>());
}

#[test]
fn insert_new_component_does_not_drop_existing_components() {
    let counter_existing = Arc::new(AtomicUsize::new(0));
    let _counter_new = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Droppable::new(&counter_existing),));

    // Insert a new component (causing archetype migration).
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // The existing Droppable should NOT have been dropped.
    assert_eq!(counter_existing.load(Ordering::Relaxed), 0);
    // Both components should be accessible.
    assert!(world.get::<Droppable>(entity).is_some());
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
}

#[test]
fn commands_insert_non_copy_then_apply() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((Position { x: 0.0, y: 0.0 },));

    let mut cmds = CommandBuffer::new();
    cmds.insert(entity, Droppable::new(&counter));
    cmds.apply(&mut world);

    // Component should be on the entity now, not dropped.
    assert_eq!(counter.load(Ordering::Relaxed), 0);
    assert!(world.get::<Droppable>(entity).is_some());

    // Despawn should call drop.
    world.despawn(entity);
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn commands_insert_value_dropped_when_not_consumed() {
    let counter = Arc::new(AtomicUsize::new(0));
    {
        let mut cmds = CommandBuffer::new();
        let entity = super::EntityId::new(9999, 0); // non-existent
        cmds.insert(entity, Droppable::new(&counter));
        // Drop cmds without applying — the InsertValue should drop its payload.
    }
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[repr(align(128))]
struct HighAlignDroppable {
    drop_count: Arc<AtomicUsize>,
    misaligned_count: Arc<AtomicUsize>,
}

impl Drop for HighAlignDroppable {
    fn drop(&mut self) {
        let address = self as *const Self as usize;
        if !address.is_multiple_of(std::mem::align_of::<Self>()) {
            self.misaligned_count.fetch_add(1, Ordering::Relaxed);
        }
        self.drop_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn commands_insert_value_drop_preserves_component_alignment() {
    let drop_count = Arc::new(AtomicUsize::new(0));
    let misaligned_count = Arc::new(AtomicUsize::new(0));
    {
        let mut cmds = CommandBuffer::new();
        cmds.insert(
            super::EntityId::new(9999, 0),
            HighAlignDroppable {
                drop_count: drop_count.clone(),
                misaligned_count: misaligned_count.clone(),
            },
        );
    }

    assert_eq!(drop_count.load(Ordering::Relaxed), 1);
    assert_eq!(misaligned_count.load(Ordering::Relaxed), 0);
}

#[test]
fn copy_types_unaffected_by_drop_machinery() {
    // Verify Copy types still work exactly as before.
    let mut world = World::new();
    let entity = world.spawn((Position { x: 1.0, y: 2.0 }, Velocity { x: 3.0, y: 4.0 }));

    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 1.0, y: 2.0 })
    );
    world.insert(entity, Position { x: 5.0, y: 6.0 });
    assert_eq!(
        world.get::<Position>(entity),
        Some(&Position { x: 5.0, y: 6.0 })
    );
    world.remove::<Velocity>(entity);
    assert_eq!(world.get::<Velocity>(entity), None);
    world.despawn(entity);
    assert!(!world.contains(entity));
}

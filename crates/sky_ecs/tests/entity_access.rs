use sky_ecs::{EntityId, World};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker;

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Large([u8; 4 * 1024]);

#[test]
fn reads_components_across_archetypes_and_reports_missing_components() {
    let mut world = World::new();
    let position_only = world.spawn((Position(1),));
    let both = world.spawn((Position(2), Velocity(3)));
    let velocity_only = world.spawn((Velocity(4),));

    let positions = world.accessor::<Position>();

    assert_eq!(positions.get(position_only), Some(&Position(1)));
    assert_eq!(positions.get(both), Some(&Position(2)));
    assert_eq!(positions.get(velocity_only), None);
}

#[test]
fn rejects_invalid_and_stale_entity_ids() {
    let mut world = World::new();
    let stale = world.spawn((Position(1),));
    assert!(world.despawn(stale));
    let current = world.spawn((Position(2),));
    assert_eq!(stale.index(), current.index());

    let positions = world.accessor::<Position>();

    assert_eq!(positions.get(stale), None);
    assert_eq!(positions.get(EntityId::new(u32::MAX, 0)), None);
    assert_eq!(positions.get(current), Some(&Position(2)));
}

#[test]
fn reads_components_across_multiple_chunk_sizes() {
    let mut world = World::new();
    let entities: Vec<_> = (0..160)
        .map(|index| world.spawn((Large([index as u8; 4 * 1024]), Position(index))))
        .collect();

    let positions = world.accessor::<Position>();

    for (index, entity) in entities.into_iter().enumerate() {
        assert_eq!(positions.get(entity), Some(&Position(index as u32)));
    }
}

#[test]
fn reads_zero_sized_components() {
    let mut world = World::new();
    let entity = world.spawn((Marker, Position(1)));

    assert_eq!(world.accessor::<Marker>().get(entity), Some(&Marker));
}

struct Droppable {
    drops: Arc<AtomicUsize>,
}

impl Drop for Droppable {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn reads_non_copy_components_without_affecting_drop_semantics() {
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut world = World::new();
        let entity = world.spawn((Droppable {
            drops: Arc::clone(&drops),
        },));

        {
            let components = world.accessor::<Droppable>();
            assert!(Arc::ptr_eq(&components.get(entity).unwrap().drops, &drops));
            assert_eq!(drops.load(Ordering::Relaxed), 0);
        }
    }

    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn mutably_updates_components_across_archetypes() {
    let mut world = World::new();
    let position_only = world.spawn((Position(1),));
    let both = world.spawn((Position(2), Velocity(3)));
    let velocity_only = world.spawn((Velocity(4),));

    {
        let mut positions = world.accessor_mut::<Position>();
        positions.get_mut(position_only).unwrap().0 += 10;
        positions.get_mut(both).unwrap().0 += 20;
        assert!(positions.get_mut(velocity_only).is_none());
    }

    assert_eq!(world.get::<Position>(position_only), Some(&Position(11)));
    assert_eq!(world.get::<Position>(both), Some(&Position(22)));
}

#[test]
fn mutable_accessor_rejects_invalid_and_stale_entity_ids() {
    let mut world = World::new();
    let stale = world.spawn((Position(1),));
    assert!(world.despawn(stale));
    let current = world.spawn((Position(2),));

    let mut positions = world.accessor_mut::<Position>();

    assert!(positions.get_mut(stale).is_none());
    assert!(positions.get_mut(EntityId::new(u32::MAX, 0)).is_none());
    positions.get_mut(current).unwrap().0 = 5;
    assert_eq!(
        positions.get_mut(current).map(|position| position.0),
        Some(5)
    );
}

#[test]
fn mutable_accessor_handles_multiple_chunk_sizes() {
    let mut world = World::new();
    let entities: Vec<_> = (0..160)
        .map(|index| world.spawn((Large([index as u8; 4 * 1024]), Position(index))))
        .collect();

    {
        let mut positions = world.accessor_mut::<Position>();
        for entity in entities.iter().copied() {
            positions.get_mut(entity).unwrap().0 += 1;
        }
    }

    for (index, entity) in entities.into_iter().enumerate() {
        assert_eq!(
            world.get::<Position>(entity),
            Some(&Position(index as u32 + 1))
        );
    }
}

#[test]
fn mutable_accessor_updates_non_copy_components() {
    let mut world = World::new();
    let entity = world.spawn((String::from("Sky"),));

    {
        let mut strings = world.accessor_mut::<String>();
        strings.get_mut(entity).unwrap().push_str(" ECS");
    }

    assert_eq!(
        world.get::<String>(entity).map(String::as_str),
        Some("Sky ECS")
    );
}

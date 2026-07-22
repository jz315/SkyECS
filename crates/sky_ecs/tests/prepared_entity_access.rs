use sky_ecs::{EntityId, PrepareAccessError, World};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker;

#[repr(align(256))]
#[derive(Debug, PartialEq)]
struct Aligned(u32);

#[allow(dead_code)]
struct Large([u8; 4 * 1024]);

#[test]
fn preserves_input_order_across_archetypes_and_chunks() {
    let mut world = World::new();
    let mut entities: Vec<_> = (0..160)
        .map(|index| world.spawn((Large([index as u8; 4 * 1024]), Position(index))))
        .collect();
    let other_archetype = world.spawn((Position(999), Velocity(1)));
    entities.insert(17, other_archetype);
    entities.reverse();

    let positions = world.prepare_access::<Position>(&entities).unwrap();
    let actual: Vec<_> = positions.iter().map(|position| position.0).collect();
    let expected: Vec<_> = entities
        .iter()
        .map(|&entity| world.get::<Position>(entity).unwrap().0)
        .collect();

    assert_eq!(actual, expected);
    assert_eq!(positions.len(), entities.len());
    assert_eq!(
        positions.get(0).map(|position| position.0),
        actual.first().copied()
    );
    assert!(positions.get(entities.len()).is_none());
}

#[test]
fn resolves_repaired_routes_after_swap_remove_and_migration() {
    let mut world = World::new();
    let migrated = world.spawn((Position(10),));
    let removed = world.spawn((Position(20),));
    let moved_row = world.spawn((Position(30),));

    assert!(world.despawn(removed));
    assert!(world.insert(migrated, Velocity(1)));

    let positions = world
        .prepare_access::<Position>(&[moved_row, migrated])
        .unwrap();
    assert_eq!(
        positions
            .iter()
            .map(|position| position.0)
            .collect::<Vec<_>>(),
        [30, 10]
    );
}

#[test]
fn accepts_empty_and_duplicate_read_sequences() {
    let mut world = World::new();
    let entity = world.spawn((Position(7),));

    let empty = world.prepare_access::<Position>(&[]).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.iter().count(), 0);

    let repeated = world
        .prepare_access::<Position>(&[entity, entity, entity])
        .unwrap();
    assert_eq!(
        repeated
            .iter()
            .map(|position| position.0)
            .collect::<Vec<_>>(),
        [7, 7, 7]
    );
}

#[test]
fn reports_the_first_invalid_or_missing_entity() {
    let mut world = World::new();
    let position = world.spawn((Position(1),));
    let missing = world.spawn((Velocity(2),));
    let stale = world.spawn((Position(3),));
    assert!(world.despawn(stale));

    assert_eq!(
        world
            .prepare_access::<Position>(&[position, missing, stale])
            .err(),
        Some(PrepareAccessError::MissingComponent {
            index: 1,
            entity: missing,
        })
    );
    assert_eq!(
        world.prepare_access::<Position>(&[position, stale]).err(),
        Some(PrepareAccessError::InvalidEntity {
            index: 1,
            entity: stale,
        })
    );

    let invalid = EntityId::new(u32::MAX, 0);
    assert_eq!(
        world.prepare_access::<Position>(&[invalid]).err(),
        Some(PrepareAccessError::InvalidEntity {
            index: 0,
            entity: invalid,
        })
    );
}

#[test]
fn reads_zero_sized_over_aligned_and_non_copy_components() {
    let mut world = World::new();
    let marker_entities = [world.spawn((Marker,)), world.spawn((Marker, Position(1)))];
    let aligned_entities = [world.spawn((Aligned(11),)), world.spawn((Aligned(22),))];
    let string_entities = [
        world.spawn((String::from("Sky"),)),
        world.spawn((String::from("ECS"),)),
    ];

    assert_eq!(
        world
            .prepare_access::<Marker>(&marker_entities)
            .unwrap()
            .iter()
            .count(),
        2
    );
    assert_eq!(
        world
            .prepare_access::<Aligned>(&aligned_entities)
            .unwrap()
            .iter()
            .map(|value| value.0)
            .collect::<Vec<_>>(),
        [11, 22]
    );
    assert_eq!(
        world
            .prepare_access::<String>(&string_entities)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["Sky", "ECS"]
    );
}

#[test]
fn mutable_plan_updates_in_prepared_order() {
    let mut world = World::new();
    let entities = [
        world.spawn((Position(1),)),
        world.spawn((Position(2), Velocity(1))),
        world.spawn((Position(3),)),
    ];
    let order = [entities[2], entities[0], entities[1]];

    {
        let mut positions = world.prepare_access_mut::<Position>(&order).unwrap();
        assert_eq!(positions.len(), 3);
        assert!(!positions.is_empty());
        assert_eq!(positions.get(0).map(|position| position.0), Some(3));
        assert_eq!(
            positions
                .iter()
                .map(|position| position.0)
                .collect::<Vec<_>>(),
            [3, 1, 2]
        );
        for (offset, position) in positions.iter_mut().enumerate() {
            position.0 += (offset as u32 + 1) * 10;
        }
        positions.get_mut(1).unwrap().0 += 1;
        assert!(positions.get_mut(3).is_none());
    }

    assert_eq!(world.get::<Position>(entities[0]), Some(&Position(22)));
    assert_eq!(world.get::<Position>(entities[1]), Some(&Position(32)));
    assert_eq!(world.get::<Position>(entities[2]), Some(&Position(13)));
}

#[test]
fn mutable_plan_rejects_duplicates_with_both_indices() {
    let mut world = World::new();
    let first = world.spawn((Position(1),));
    let second = world.spawn((Position(2),));

    assert_eq!(
        world
            .prepare_access_mut::<Position>(&[first, second, first])
            .err(),
        Some(PrepareAccessError::DuplicateEntity {
            first_index: 0,
            duplicate_index: 2,
            entity: first,
        })
    );
}

#[test]
fn mutable_plan_validates_before_reporting_later_duplicates() {
    let mut world = World::new();
    let first = world.spawn((Position(1),));
    let missing = world.spawn((Velocity(2),));

    assert_eq!(
        world
            .prepare_access_mut::<Position>(&[first, missing, first])
            .err(),
        Some(PrepareAccessError::MissingComponent {
            index: 1,
            entity: missing,
        })
    );
}

#[test]
fn mutable_plan_handles_distinct_zero_sized_and_over_aligned_components() {
    let mut world = World::new();
    let markers = [world.spawn((Marker,)), world.spawn((Marker,))];
    let aligned = [world.spawn((Aligned(4),)), world.spawn((Aligned(8),))];

    assert_eq!(
        world
            .prepare_access_mut::<Marker>(&markers)
            .unwrap()
            .iter_mut()
            .count(),
        2
    );
    {
        let mut values = world.prepare_access_mut::<Aligned>(&aligned).unwrap();
        for value in values.iter_mut() {
            value.0 *= 2;
        }
    }

    assert_eq!(world.get::<Aligned>(aligned[0]), Some(&Aligned(8)));
    assert_eq!(world.get::<Aligned>(aligned[1]), Some(&Aligned(16)));
}

#[test]
fn mutable_plan_updates_non_copy_components_without_changing_ownership() {
    let mut world = World::new();
    let entities = [
        world.spawn((String::from("Sky"),)),
        world.spawn((String::from("modern"),)),
    ];

    {
        let mut strings = world.prepare_access_mut::<String>(&entities).unwrap();
        strings.get_mut(0).unwrap().push_str(" ECS");
        strings.get_mut(1).unwrap().push_str(" access");
    }

    assert_eq!(world.get::<String>(entities[0]).unwrap(), "Sky ECS");
    assert_eq!(world.get::<String>(entities[1]).unwrap(), "modern access");
}

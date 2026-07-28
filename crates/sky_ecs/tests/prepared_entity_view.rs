use sky_ecs::{EntityId, PreparedEntityView, QueryData, World};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(u32);

#[derive(Debug)]
struct Marker;

#[repr(align(64))]
#[derive(Debug, PartialEq)]
struct Aligned(u32);

#[derive(Debug)]
struct OwnedValue {
    text: String,
    drops: Arc<AtomicUsize>,
}

impl Drop for OwnedValue {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

#[test]
fn shared_tuple_access_handles_archetypes_stale_ids_and_missing_components() {
    let mut world = World::new();
    let both = world.spawn((Position(1), Velocity(2)));
    let position_only = world.spawn((Position(3),));
    let stale = world.spawn((Position(4), Velocity(5)));
    assert!(world.despawn(stale));

    let mut prepared = PreparedEntityView::<(&Position, &Velocity)>::new();
    let bound = prepared.bind(&world);

    assert_eq!(
        bound
            .get(both)
            .map(|(position, velocity)| (*position, *velocity)),
        Some((Position(1), Velocity(2)))
    );
    assert!(bound.get(position_only).is_none());
    assert!(bound.get(stale).is_none());
    assert!(bound.get(EntityId::new(u32::MAX, 0)).is_none());
}

#[test]
fn optional_only_queries_distinguish_live_entities_from_invalid_ids() {
    let mut world = World::new();
    let position = world.spawn((Position(1),));
    let velocity = world.spawn((Velocity(2),));
    let neither = world.spawn(("marker",));
    let stale = world.spawn((Position(3),));
    assert!(world.despawn(stale));

    let mut single = PreparedEntityView::<Option<&Position>>::new();
    let bound = single.bind(&world);
    assert_eq!(bound.get(position), Some(Some(&Position(1))));
    assert_eq!(bound.get(velocity), Some(None));
    assert_eq!(bound.get(stale), None);

    let mut tuple = PreparedEntityView::<(Option<&Position>, Option<&Velocity>)>::new();
    let bound = tuple.bind(&world);
    assert_eq!(bound.get(neither), Some((None, None)));
}

#[test]
fn mutable_and_derived_queries_fetch_one_row_directly() {
    let mut world = World::new();
    let first = world.spawn((Position(1), Velocity(10)));
    let second = world.spawn((Position(2), Velocity(20)));

    let mut prepared = PreparedEntityView::<Movement>::new();
    {
        let mut bound = prepared.bind_mut(&mut world);
        let item = bound.get_mut(first).unwrap();
        item.position.0 += item.velocity.0;
        let item = bound.get_mut(second).unwrap();
        item.position.0 += item.velocity.0;
    }

    assert_eq!(world.get::<Position>(first), Some(&Position(11)));
    assert_eq!(world.get::<Position>(second), Some(&Position(22)));
}

#[test]
fn prepared_view_refreshes_after_clear_and_when_switching_worlds() {
    let mut first_world = World::new();
    let first_entities: Vec<_> = (0..300)
        .map(|value| first_world.spawn((Position(value), [value; 128])))
        .collect();
    let mut prepared = PreparedEntityView::<&Position>::new();
    assert_eq!(
        prepared.bind(&first_world).get(first_entities[299]),
        Some(&Position(299))
    );

    first_world.clear();
    let after_clear = first_world.spawn((Position(900),));
    assert_eq!(
        prepared.bind(&first_world).get(after_clear),
        Some(&Position(900))
    );

    let mut second_world = World::new();
    let second = second_world.spawn((Position(42), Velocity(1)));
    assert_eq!(
        prepared.bind(&second_world).get(second),
        Some(&Position(42))
    );
}

#[test]
fn mutable_tuple_handles_zero_sized_over_aligned_and_non_copy_components() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let entity = world.spawn((
        Marker,
        Aligned(64),
        OwnedValue {
            text: String::from("owned"),
            drops: drops.clone(),
        },
    ));
    let mut prepared = PreparedEntityView::<(&Marker, &mut Aligned, &mut OwnedValue)>::new();

    {
        let mut bound = prepared.bind_mut(&mut world);
        let (marker, aligned, owned) = bound.get_mut(entity).unwrap();
        let _ = marker;
        aligned.0 += 1;
        owned.text.push_str(" value");
    }

    assert_eq!(world.get::<Aligned>(entity), Some(&Aligned(65)));
    assert_eq!(world.get::<OwnedValue>(entity).unwrap().text, "owned value");
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(world);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

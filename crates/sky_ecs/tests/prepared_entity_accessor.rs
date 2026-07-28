use sky_ecs::{EntityId, PreparedEntityAccessor, World};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(u32);

#[derive(Debug, PartialEq)]
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

#[test]
fn shared_access_handles_archetypes_missing_components_and_stale_ids() {
    let mut world = World::new();
    let position_only = world.spawn((Position(1),));
    let both = world.spawn((Position(2), Velocity(3)));
    let missing = world.spawn((Velocity(4),));
    let stale = world.spawn((Position(5),));
    assert!(world.despawn(stale));

    let mut prepared = PreparedEntityAccessor::<Position>::new();
    let positions = prepared.bind(&world);

    assert_eq!(positions.get(position_only), Some(&Position(1)));
    assert_eq!(positions.get(both), Some(&Position(2)));
    assert_eq!(positions.get(missing), None);
    assert_eq!(positions.get(stale), None);
    assert_eq!(positions.get(EntityId::new(u32::MAX, 0)), None);
}

#[test]
fn mutable_access_handles_zst_alignment_and_non_copy_ownership() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let marker = world.spawn((Marker,));
    let aligned = world.spawn((Aligned(64),));
    let owned = world.spawn((OwnedValue {
        text: String::from("Sky"),
        drops: Arc::clone(&drops),
    },));

    let mut markers = PreparedEntityAccessor::<Marker>::new();
    assert_eq!(markers.bind(&world).get(marker), Some(&Marker));

    let mut aligned_values = PreparedEntityAccessor::<Aligned>::new();
    {
        let mut values = aligned_values.bind_mut(&mut world);
        values.get_mut(aligned).unwrap().0 += 1;
    }
    assert_eq!(world.get::<Aligned>(aligned), Some(&Aligned(65)));

    let mut owned_values = PreparedEntityAccessor::<OwnedValue>::new();
    {
        let mut values = owned_values.bind_mut(&mut world);
        values.get_mut(owned).unwrap().text.push_str(" ECS");
    }
    assert_eq!(world.get::<OwnedValue>(owned).unwrap().text, "Sky ECS");
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(world);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn one_plan_can_bind_read_only_then_mutably_across_worlds() {
    let mut first_world = World::new();
    let first = first_world.spawn((Position(1),));
    let mut prepared = PreparedEntityAccessor::<Position>::new();
    assert_eq!(prepared.bind(&first_world).get(first), Some(&Position(1)));

    let mut second_world = World::new();
    let second = second_world.spawn((Position(2),));
    {
        let mut positions = prepared.bind_mut(&mut second_world);
        positions.get_mut(second).unwrap().0 = 3;
    }
    assert_eq!(second_world.get::<Position>(second), Some(&Position(3)));
    assert_eq!(prepared.cache_stats().rebuild_count, 2);
}

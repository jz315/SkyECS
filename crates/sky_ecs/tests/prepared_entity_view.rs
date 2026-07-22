use sky_ecs::ecs::__private::{Chunk, QueryDescriptor, QuerySpec, ReadOnlyQuerySpec};
use sky_ecs::{EntityId, PreparedEntityView, QueryData, World};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(u32);

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

struct LegacyPosition<'w>(&'w Position);

unsafe impl QuerySpec for LegacyPosition<'static> {
    type Chunk<'w> = <&'static Position as QuerySpec>::Chunk<'w>;
    type Item<'w> = LegacyPosition<'w>;

    fn descriptor() -> QueryDescriptor {
        <&'static Position as QuerySpec>::descriptor()
    }

    unsafe fn chunk_from_raw<'w>(chunk: &'w Chunk, component_indices: &[u8]) -> Self::Chunk<'w> {
        unsafe { <&'static Position as QuerySpec>::chunk_from_raw(chunk, component_indices) }
    }

    unsafe fn chunk_from_raw_parts<'w>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
    ) -> Self::Chunk<'w> {
        unsafe {
            <&'static Position as QuerySpec>::chunk_from_raw_parts(component_ptrs, start, len)
        }
    }

    unsafe fn for_each_entity_raw_parts<'w, Func>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
        f: &mut Func,
    ) where
        Func: FnMut(Self::Item<'w>),
    {
        unsafe {
            <&'static Position as QuerySpec>::for_each_entity_raw_parts(
                component_ptrs,
                start,
                len,
                &mut |position| f(LegacyPosition(position)),
            );
        }
    }

    unsafe fn for_each_entity<'w, Func>(chunk: &'w Chunk, component_indices: &[u8], f: &mut Func)
    where
        Func: FnMut(Self::Item<'w>),
    {
        unsafe {
            <&'static Position as QuerySpec>::for_each_entity(
                chunk,
                component_indices,
                &mut |position| f(LegacyPosition(position)),
            );
        }
    }
}

unsafe impl ReadOnlyQuerySpec for LegacyPosition<'static> {}

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
fn external_query_spec_uses_the_compatible_default_single_row_path() {
    let mut world = World::new();
    let entity = world.spawn((Position(7),));
    let mut prepared = PreparedEntityView::<LegacyPosition>::new();

    assert_eq!(prepared.bind(&world).get(entity).unwrap().0, &Position(7));
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

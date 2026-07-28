use super::entity_records::EntityRouteView;
use crate::ecs::{EntityFetchSpec, EntityId, ReadOnlyQuerySpec, World};
use core::marker::PhantomData;

mod cache;

pub(crate) use cache::EntityViewCache;
pub use cache::EntityViewCacheStats;

/// A reusable tuple-capable component view for arbitrary entity IDs.
///
/// Binding refreshes the route table from the current world while retaining
/// its allocations. Each lookup validates the entity generation and resolves
/// all query components from one entity route.
#[must_use = "prepared entity views do nothing until they are bound"]
pub struct PreparedEntityView<Q: EntityFetchSpec> {
    cache: EntityViewCache<Q>,
}

impl<Q: EntityFetchSpec> Default for PreparedEntityView<Q> {
    fn default() -> Self {
        Self {
            cache: EntityViewCache::default(),
        }
    }
}

impl<Q: EntityFetchSpec> PreparedEntityView<Q> {
    /// Creates an empty reusable entity view.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns route-cache diagnostics without binding the view.
    pub fn cache_stats(&self) -> EntityViewCacheStats {
        self.cache.stats()
    }

    /// Binds this plan exclusively to `world`.
    ///
    /// Binding refreshes component bases after a queried component's bases or
    /// the World's route-table shape changes; otherwise the existing route
    /// table is reused. Unrelated archetype backing changes do not rebuild it.
    ///
    /// ```compile_fail
    /// use sky_ecs::{PreparedEntityView, World};
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((1_u32,));
    /// let mut prepared = PreparedEntityView::<&mut u32>::new();
    /// let mut bound = prepared.bind_mut(&mut world);
    /// world.spawn((2_u32,));
    /// let _ = bound.get_mut(entity);
    /// ```
    #[inline]
    pub fn bind_mut<'s, 'w>(&'s mut self, world: &'w mut World) -> BoundEntityViewMut<'s, 'w, Q> {
        self.cache.prepare(world);
        BoundEntityViewMut {
            entity_routes: EntityRouteView::new(world),
            cache: &self.cache,
            world_marker: PhantomData,
        }
    }
}

impl<Q: EntityFetchSpec + ReadOnlyQuerySpec> PreparedEntityView<Q> {
    /// Binds this plan immutably to `world`.
    #[inline]
    pub fn bind<'s, 'w>(&'s mut self, world: &'w World) -> BoundEntityView<'s, 'w, Q> {
        self.cache.prepare(world);
        BoundEntityView {
            entity_routes: EntityRouteView::new(world),
            cache: &self.cache,
        }
    }
}

/// A read-only prepared entity view bound to one world.
#[must_use = "bound entity views do nothing until get is called"]
pub struct BoundEntityView<'s, 'w, Q: EntityFetchSpec> {
    entity_routes: EntityRouteView<'w>,
    cache: &'s EntityViewCache<Q>,
}

impl<Q: EntityFetchSpec + ReadOnlyQuerySpec> BoundEntityView<'_, '_, Q> {
    /// Returns all requested components for a live matching entity.
    ///
    /// Optional-only queries return `Some` for every live entity, even when
    /// every component item inside the query is `None`.
    ///
    /// ```compile_fail
    /// use sky_ecs::{BoundEntityView, EntityId};
    ///
    /// fn leak(
    ///     view: &BoundEntityView<'_, '_, &'static u32>,
    ///     entity: EntityId,
    /// ) -> &'static u32 {
    ///     view.get(entity).unwrap()
    /// }
    /// ```
    #[inline(always)]
    pub fn get<'a>(&'a self, entity: EntityId) -> Option<Q::Item<'a>> {
        let route = self.entity_routes.resolve(entity)?;
        let (fetch, entity_index) = self.cache.row(route)?;
        Some(unsafe {
            // SAFETY: prepare wrote Q's typed fetch for this live route. The
            // shared bound World prevents pointer relocation and Q is
            // read-only, so all references may share this borrow.
            Q::fetch_item(fetch, entity_index)
        })
    }
}

/// An exclusive prepared entity view bound to one world.
#[must_use = "bound mutable entity views do nothing until get_mut is called"]
pub struct BoundEntityViewMut<'s, 'w, Q: EntityFetchSpec> {
    entity_routes: EntityRouteView<'w>,
    cache: &'s EntityViewCache<Q>,
    world_marker: PhantomData<&'w mut World>,
}

impl<Q: EntityFetchSpec> BoundEntityViewMut<'_, '_, Q> {
    /// Returns all requested components for a live matching entity.
    ///
    /// The returned item is tied to the exclusive borrow of this bound view,
    /// preventing a second lookup while mutable references remain live.
    ///
    /// ```compile_fail
    /// use sky_ecs::{PreparedEntityView, World};
    ///
    /// let mut world = World::new();
    /// let first_entity = world.spawn((1_u32,));
    /// let second_entity = world.spawn((2_u32,));
    /// let mut prepared = PreparedEntityView::<&mut u32>::new();
    /// let mut bound = prepared.bind_mut(&mut world);
    /// let first = bound.get_mut(first_entity).unwrap();
    /// let second = bound.get_mut(second_entity).unwrap();
    /// *first += *second;
    /// ```
    #[inline(always)]
    pub fn get_mut<'a>(&'a mut self, entity: EntityId) -> Option<Q::Item<'a>> {
        let route = self.entity_routes.resolve(entity)?;
        let (fetch, entity_index) = self.cache.row(route)?;
        Some(unsafe {
            // SAFETY: bind_mut exclusively borrows the World, prepare wrote
            // Q's complete typed fetch, and this method's mutable borrow
            // prevents overlapping query items from the same view.
            Q::fetch_item(fetch, entity_index)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position(u32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity(u32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Marker(u32);

    #[allow(dead_code)]
    struct Large([u8; 4 * 1024]);

    #[test]
    fn route_tables_reuse_allocations_across_binds() {
        let mut world = World::new();
        let entity = world.spawn((Position(1), Velocity(2)));
        let mut prepared = PreparedEntityView::<(&Position, &Velocity)>::new();

        assert_eq!(prepared.bind(&world).get(entity).unwrap().0 .0, 1);
        let matched_ptr = prepared.cache.matched_routes.as_ptr();
        let matched_capacity = prepared.cache.matched_routes.capacity();
        let fetch_ptr = prepared.cache.fetches.as_ptr();
        let fetch_capacity = prepared.cache.fetches.capacity();

        assert_eq!(prepared.bind(&world).get(entity).unwrap().1 .0, 2);
        let stats = prepared.cache_stats();
        assert_eq!(stats.rebuild_count, 1);
        assert_eq!(stats.fetch_slots, stats.route_slots);
        assert_eq!(prepared.cache.matched_routes.as_ptr(), matched_ptr);
        assert_eq!(prepared.cache.matched_routes.capacity(), matched_capacity);
        assert_eq!(prepared.cache.fetches.as_ptr(), fetch_ptr);
        assert_eq!(prepared.cache.fetches.capacity(), fetch_capacity);
    }

    #[test]
    fn row_churn_does_not_rebuild_column_routes() {
        let mut world = World::new();
        let first = world.spawn((Position(1),));
        let mut prepared = PreparedEntityView::<&Position>::new();
        assert_eq!(prepared.bind(&world).get(first), Some(&Position(1)));
        assert_eq!(prepared.cache_stats().rebuild_count, 1);

        let second = world.spawn((Position(2),));
        assert!(world.despawn(first));
        assert_eq!(prepared.bind(&world).get(second), Some(&Position(2)));
        assert_eq!(prepared.cache_stats().rebuild_count, 1);
    }

    #[test]
    fn unrelated_promotion_does_not_rebuild_query_routes() {
        let mut world = World::new();
        let entity = world.spawn((Position(1), Velocity(2)));
        world.spawn((Marker(1),));
        let mut prepared = PreparedEntityView::<(&Position, &Velocity)>::new();
        assert_eq!(
            prepared.bind(&world).get(entity),
            Some((&Position(1), &Velocity(2)))
        );

        let route_slots = world.chunk_route_slot_count();
        let marker = crate::ecs::component_type::<Marker>();
        let initial_marker_epoch = world.component_column_base_epoch(&marker);
        while world.component_column_base_epoch(&marker) == initial_marker_epoch {
            world.spawn((Marker(2),));
        }

        assert_eq!(world.chunk_route_slot_count(), route_slots);
        assert_eq!(
            prepared.bind(&world).get(entity),
            Some((&Position(1), &Velocity(2)))
        );
        assert_eq!(prepared.cache_stats().rebuild_count, 1);
    }

    #[test]
    fn optional_only_view_rebuilds_when_route_slots_grow() {
        let mut world = World::new();
        let position = world.spawn((Position(1),));
        let mut prepared = PreparedEntityView::<Option<&Position>>::new();
        assert_eq!(
            prepared.bind(&world).get(position),
            Some(Some(&Position(1)))
        );

        let velocity = world.spawn((Velocity(2),));
        assert_eq!(prepared.bind(&world).get(velocity), Some(None));
        assert_eq!(prepared.cache_stats().rebuild_count, 2);
        assert_eq!(
            prepared.cache_stats().route_slots,
            world.chunk_route_slot_count()
        );
    }

    #[test]
    fn bind_reacquires_reallocated_entity_records_without_rebuilding_routes() {
        let mut world = World::new();
        let first = world.spawn((Position(1),));
        let mut prepared = PreparedEntityView::<&Position>::new();

        let initial_records = {
            let bound = prepared.bind(&world);
            assert_eq!(bound.get(first), Some(&Position(1)));
            bound.entity_routes.as_ptr()
        };
        let initial_column_base_epoch = world.column_base_epoch();
        assert_eq!(prepared.cache_stats().rebuild_count, 1);

        let newest = loop {
            let entity = world.spawn((Position(2),));
            if EntityRouteView::new(&world).as_ptr() != initial_records {
                break entity;
            }
        };

        assert_eq!(world.column_base_epoch(), initial_column_base_epoch);
        let bound = prepared.bind(&world);
        assert_ne!(bound.entity_routes.as_ptr(), initial_records);
        assert_eq!(bound.get(first), Some(&Position(1)));
        assert_eq!(bound.get(newest), Some(&Position(2)));
        assert_eq!(prepared.cache_stats().rebuild_count, 1);
    }

    #[test]
    fn explicit_route_shrink_rebuilds_to_the_shorter_table() {
        let mut world = World::new();
        let survivor = world.spawn((Position(1),));
        let temporary: Vec<_> = (0..160)
            .map(|value| world.spawn((Position(value), Large([value as u8; 4 * 1024]))))
            .collect();
        let mut prepared = PreparedEntityView::<&Position>::new();
        assert_eq!(prepared.bind(&world).get(survivor), Some(&Position(1)));
        let peak = world.route_table_stats();
        for entity in temporary {
            assert!(world.despawn(entity));
        }
        assert!(world.route_table_stats().vacant_route_slots > 0);
        let shrunk = world.shrink_route_tables();
        assert!(shrunk.route_slots < peak.route_slots);
        assert_eq!(prepared.bind(&world).get(survivor), Some(&Position(1)));
        assert_eq!(prepared.cache_stats().route_slots, shrunk.route_slots);
        assert_eq!(prepared.cache_stats().rebuild_count, 2);
    }

    #[test]
    fn route_tables_reuse_capacity_after_clear_shrink_and_regrow() {
        let mut world = World::new();
        for value in 0..160 {
            world.spawn((Position(value), Large([value as u8; 4 * 1024])));
        }
        let mut prepared = PreparedEntityView::<&Position>::new();
        let _ = prepared.bind(&world);
        assert!(prepared.cache.matched_routes.len() > 1);
        let matched_ptr = prepared.cache.matched_routes.as_ptr();
        let matched_capacity = prepared.cache.matched_routes.capacity();
        let fetch_ptr = prepared.cache.fetches.as_ptr();
        let fetch_capacity = prepared.cache.fetches.capacity();

        world.clear();
        let _ = prepared.bind(&world);
        assert!(prepared.cache.matched_routes.is_empty());

        let mut last = None;
        for value in 0..160 {
            last = Some(world.spawn((Position(value), Large([value as u8; 4 * 1024]))));
        }
        let last = last.unwrap();
        assert_eq!(prepared.bind(&world).get(last), Some(&Position(159)));
        assert_eq!(prepared.cache.matched_routes.as_ptr(), matched_ptr);
        assert_eq!(prepared.cache.matched_routes.capacity(), matched_capacity);
        assert_eq!(prepared.cache.fetches.as_ptr(), fetch_ptr);
        assert_eq!(prepared.cache.fetches.capacity(), fetch_capacity);
    }

    #[test]
    fn refreshes_pointer_when_tiny_chunk_keeps_id_during_promotion() {
        let mut world = World::new();
        let first = world.spawn((Position(1),));
        let initial_route = world.entity_route(first).unwrap();
        let initial_location = world.entity_location(first).unwrap();
        let component_index = world.data[initial_location.data_index]
            .archetype
            .query_component_index(&crate::ecs::component_type::<Position>())
            .unwrap();
        let initial_pointer = world.data[initial_location.data_index].chunks
            [initial_location.chunk_index]
            .column_ptr(component_index);

        let mut prepared = PreparedEntityView::<&Position>::new();
        assert_eq!(prepared.bind(&world).get(first), Some(&Position(1)));

        let (newest, promoted_pointer) = loop {
            let newest = world.spawn((Position(2),));
            let location = world.entity_location(first).unwrap();
            let pointer = world.data[location.data_index].chunks[location.chunk_index]
                .column_ptr(component_index);
            if pointer != initial_pointer {
                break (newest, pointer);
            }
        };

        assert_ne!(promoted_pointer, initial_pointer);
        assert_eq!(
            world.entity_route(first).unwrap().chunk_id,
            initial_route.chunk_id
        );
        let bound = prepared.bind(&world);
        assert_eq!(bound.get(first), Some(&Position(1)));
        assert_eq!(bound.get(newest), Some(&Position(2)));
        assert_eq!(prepared.cache_stats().rebuild_count, 2);
    }

    #[test]
    fn refreshes_recycled_routes_in_both_match_directions() {
        let mut world = World::new();
        let matching = world.spawn((Position(1),));
        let matching_route = world.entity_route(matching).unwrap().chunk_id;
        let mut prepared = PreparedEntityView::<&Position>::new();
        assert!(prepared.bind(&world).get(matching).is_some());
        assert!(world.despawn(matching));

        let non_matching = world.spawn((Velocity(2),));
        assert_eq!(
            world.entity_route(non_matching).unwrap().chunk_id,
            matching_route
        );
        assert!(prepared.bind(&world).get(non_matching).is_none());
        assert!(world.despawn(non_matching));

        let matching_again = world.spawn((Position(3),));
        assert_eq!(
            world.entity_route(matching_again).unwrap().chunk_id,
            matching_route
        );
        assert_eq!(
            prepared.bind(&world).get(matching_again),
            Some(&Position(3))
        );
    }
}

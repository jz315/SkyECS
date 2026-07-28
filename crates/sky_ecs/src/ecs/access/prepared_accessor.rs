use super::entity_records::EntityRouteView;
use super::routes::{refresh_component_routes, resolve_component_route};
use crate::ecs::{component_type, ComponentType, EntityId, World};
use core::marker::PhantomData;
use core::ptr::NonNull;
use std::sync::Arc;

/// Diagnostic counters for a reusable single-component route cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntityAccessorCacheStats {
    /// Number of times component column routes have been rebuilt.
    pub rebuild_count: u64,
    /// Number of chunk-route slots represented by the cache.
    pub route_slots: usize,
}

/// A reusable single-component route plan for arbitrary entity IDs.
///
/// Unlike [`EntityAccessor`](super::EntityAccessor), this plan retains its
/// route-table allocation and resolved component bases across binds. It
/// rebuilds only after switching Worlds, when `T` column bases change, or when
/// the World's route-table shape changes. Backing changes for unrelated
/// components do not invalidate the plan.
#[must_use = "prepared entity accessors do nothing until they are bound"]
pub struct PreparedEntityAccessor<T> {
    component: ComponentType,
    columns: Vec<Option<NonNull<T>>>,
    cached_world: Option<Arc<()>>,
    cached_component_column_base_epoch: Option<u64>,
    cached_route_table_epoch: Option<u64>,
    rebuild_count: u64,
}

impl<T: 'static> Default for PreparedEntityAccessor<T> {
    fn default() -> Self {
        Self {
            component: component_type::<T>(),
            columns: Vec::new(),
            cached_world: None,
            cached_component_column_base_epoch: None,
            cached_route_table_epoch: None,
            rebuild_count: 0,
        }
    }
}

impl<T: 'static> PreparedEntityAccessor<T> {
    /// Creates an empty reusable single-component route plan.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns route-cache diagnostics without binding the plan.
    pub fn cache_stats(&self) -> EntityAccessorCacheStats {
        EntityAccessorCacheStats {
            rebuild_count: self.rebuild_count,
            route_slots: self.columns.len(),
        }
    }

    /// Binds this plan immutably to `world`.
    #[inline]
    pub fn bind<'s, 'w>(&'s mut self, world: &'w World) -> BoundEntityAccessor<'s, 'w, T> {
        self.prepare(world);
        BoundEntityAccessor {
            entity_routes: EntityRouteView::new(world),
            columns: &self.columns,
        }
    }

    /// Binds this plan exclusively to `world`.
    ///
    /// Structural mutation is rejected while the bound accessor remains in
    /// use:
    ///
    /// ```compile_fail
    /// use sky_ecs::{PreparedEntityAccessor, World};
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((1_u32,));
    /// let mut prepared = PreparedEntityAccessor::<u32>::new();
    /// let mut bound = prepared.bind_mut(&mut world);
    /// world.spawn((2_u32,));
    /// let _ = bound.get_mut(entity);
    /// ```
    #[inline]
    pub fn bind_mut<'s, 'w>(
        &'s mut self,
        world: &'w mut World,
    ) -> BoundEntityAccessorMut<'s, 'w, T> {
        self.prepare(world);
        BoundEntityAccessorMut {
            entity_routes: EntityRouteView::new(world),
            columns: &self.columns,
            world_marker: PhantomData,
        }
    }

    #[inline]
    fn prepare(&mut self, world: &World) {
        let same_world = self
            .cached_world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        let component_column_base_epoch = world.component_column_base_epoch(&self.component);
        let route_table_epoch = world.route_table_epoch();
        if same_world
            && self.cached_component_column_base_epoch == Some(component_column_base_epoch)
            && self.cached_route_table_epoch == Some(route_table_epoch)
        {
            return;
        }

        refresh_component_routes(&mut self.columns, world, &self.component);
        self.cached_world = Some(Arc::clone(world.cache_token()));
        self.cached_component_column_base_epoch = Some(component_column_base_epoch);
        self.cached_route_table_epoch = Some(route_table_epoch);
        self.rebuild_count = self
            .rebuild_count
            .checked_add(1)
            .expect("prepared entity-accessor rebuild counter exhausted");
    }
}

/// A reusable read-only single-component accessor bound to one World.
#[must_use = "bound entity accessors do nothing until get is called"]
pub struct BoundEntityAccessor<'s, 'w, T> {
    entity_routes: EntityRouteView<'w>,
    columns: &'s [Option<NonNull<T>>],
}

impl<T: 'static> BoundEntityAccessor<'_, '_, T> {
    /// Returns component `T` for a live matching entity.
    ///
    /// The reference cannot outlive this bound accessor:
    ///
    /// ```compile_fail
    /// use sky_ecs::{BoundEntityAccessor, EntityId};
    ///
    /// fn leak(
    ///     accessor: &BoundEntityAccessor<'_, '_, u32>,
    ///     entity: EntityId,
    /// ) -> &'static u32 {
    ///     accessor.get(entity).unwrap()
    /// }
    /// ```
    #[inline(always)]
    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let route = self.entity_routes.resolve(entity)?;
        let pointer = resolve_component_route(self.columns, route)?;
        Some(unsafe {
            // SAFETY: the route cache was prepared from this immutably
            // borrowed World, and the returned reference is tied to the
            // current bound accessor borrow.
            &*pointer.as_ptr()
        })
    }
}

/// A reusable exclusive single-component accessor bound to one World.
#[must_use = "bound mutable entity accessors do nothing until get_mut is called"]
pub struct BoundEntityAccessorMut<'s, 'w, T> {
    entity_routes: EntityRouteView<'w>,
    columns: &'s [Option<NonNull<T>>],
    world_marker: PhantomData<&'w mut World>,
}

impl<T: 'static> BoundEntityAccessorMut<'_, '_, T> {
    /// Returns component `T` with exclusive access.
    ///
    /// Mutable results cannot overlap through safe code:
    ///
    /// ```compile_fail
    /// use sky_ecs::{PreparedEntityAccessor, World};
    ///
    /// let mut world = World::new();
    /// let first_entity = world.spawn((1_u32,));
    /// let second_entity = world.spawn((2_u32,));
    /// let mut prepared = PreparedEntityAccessor::<u32>::new();
    /// let mut bound = prepared.bind_mut(&mut world);
    /// let first = bound.get_mut(first_entity).unwrap();
    /// let second = bound.get_mut(second_entity).unwrap();
    /// *first += *second;
    /// ```
    #[inline(always)]
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let route = self.entity_routes.resolve(entity)?;
        let pointer = resolve_component_route(self.columns, route)?;
        Some(unsafe {
            // SAFETY: bind_mut retains exclusive access to the originating
            // World, and this mutable accessor borrow prevents overlapping
            // component references.
            &mut *pointer.as_ptr()
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

    #[allow(dead_code)]
    struct Large([u8; 4 * 1024]);

    #[test]
    fn stable_binds_reuse_routes_and_allocation() {
        let mut world = World::new();
        let entity = world.spawn((Position(1),));
        let mut prepared = PreparedEntityAccessor::<Position>::new();

        assert_eq!(prepared.bind(&world).get(entity), Some(&Position(1)));
        let allocation = prepared.columns.as_ptr();
        let capacity = prepared.columns.capacity();
        assert_eq!(prepared.bind(&world).get(entity), Some(&Position(1)));

        assert_eq!(
            prepared.cache_stats(),
            EntityAccessorCacheStats {
                rebuild_count: 1,
                route_slots: world.chunk_route_slot_count(),
            }
        );
        assert_eq!(prepared.columns.as_ptr(), allocation);
        assert_eq!(prepared.columns.capacity(), capacity);
    }

    #[test]
    fn row_churn_reuses_columns_but_reacquires_entity_records() {
        let mut world = World::new();
        let first = world.spawn((Position(1),));
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        let initial_records = {
            let bound = prepared.bind(&world);
            assert_eq!(bound.get(first), Some(&Position(1)));
            bound.entity_routes.as_ptr()
        };

        let newest = loop {
            let entity = world.spawn((Position(2),));
            if EntityRouteView::new(&world).as_ptr() != initial_records {
                break entity;
            }
        };
        assert!(world.despawn(first));

        let bound = prepared.bind(&world);
        assert_ne!(bound.entity_routes.as_ptr(), initial_records);
        assert_eq!(bound.get(newest), Some(&Position(2)));
        assert_eq!(prepared.cache_stats().rebuild_count, 1);
    }

    #[test]
    fn unrelated_promotion_does_not_rebuild_component_routes() {
        let mut world = World::new();
        let position = world.spawn((Position(1),));
        world.spawn((Velocity(1),));
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        assert_eq!(prepared.bind(&world).get(position), Some(&Position(1)));

        let route_slots = world.chunk_route_slot_count();
        let velocity = crate::ecs::component_type::<Velocity>();
        let initial_velocity_epoch = world.component_column_base_epoch(&velocity);
        while world.component_column_base_epoch(&velocity) == initial_velocity_epoch {
            world.spawn((Velocity(2),));
        }

        assert_eq!(world.chunk_route_slot_count(), route_slots);
        assert_eq!(prepared.bind(&world).get(position), Some(&Position(1)));
        assert_eq!(prepared.cache_stats().rebuild_count, 1);
    }

    #[test]
    fn unrelated_route_growth_resizes_for_missing_component_lookups() {
        let mut world = World::new();
        let position = world.spawn((Position(1),));
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        assert_eq!(prepared.bind(&world).get(position), Some(&Position(1)));

        let velocity = world.spawn((Velocity(2),));
        assert!(prepared.bind(&world).get(velocity).is_none());
        assert_eq!(prepared.cache_stats().rebuild_count, 2);
        assert_eq!(
            prepared.cache_stats().route_slots,
            world.chunk_route_slot_count()
        );
    }

    #[test]
    fn switching_worlds_and_recycling_routes_refreshes_columns() {
        let mut first_world = World::new();
        let first = first_world.spawn((Position(1),));
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        assert_eq!(prepared.bind(&first_world).get(first), Some(&Position(1)));

        let mut second_world = World::new();
        let missing = second_world.spawn((Velocity(2),));
        assert!(prepared.bind(&second_world).get(missing).is_none());
        assert!(second_world.despawn(missing));
        let matching = second_world.spawn((Position(3),));
        assert_eq!(
            prepared.bind(&second_world).get(matching),
            Some(&Position(3))
        );
        assert!(second_world.despawn(matching));
        let missing_again = second_world.spawn((Velocity(4),));
        assert!(prepared.bind(&second_world).get(missing_again).is_none());
        assert_eq!(prepared.cache_stats().rebuild_count, 4);
    }

    #[test]
    fn tiny_promotion_refreshes_the_same_chunk_route() {
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
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        assert_eq!(prepared.bind(&world).get(first), Some(&Position(1)));

        let newest = loop {
            let newest = world.spawn((Position(2),));
            let location = world.entity_location(first).unwrap();
            let pointer = world.data[location.data_index].chunks[location.chunk_index]
                .column_ptr(component_index);
            if pointer != initial_pointer {
                break newest;
            }
        };

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
    fn clear_shrink_and_regrow_reuse_capacity() {
        let mut world = World::new();
        for value in 0..160 {
            world.spawn((Position(value), Large([value as u8; 4 * 1024])));
        }
        let mut prepared = PreparedEntityAccessor::<Position>::new();
        let _ = prepared.bind(&world);
        let allocation = prepared.columns.as_ptr();
        let capacity = prepared.columns.capacity();

        world.clear();
        let _ = world.shrink_route_tables();
        assert_eq!(prepared.bind(&world).get(EntityId::new(0, 0)), None);
        assert_eq!(prepared.cache_stats().route_slots, 0);

        let mut last = None;
        for value in 0..160 {
            last = Some(world.spawn((Position(value), Large([value as u8; 4 * 1024]))));
        }
        assert_eq!(
            prepared.bind(&world).get(last.unwrap()),
            Some(&Position(159))
        );
        assert_eq!(prepared.columns.as_ptr(), allocation);
        assert_eq!(prepared.columns.capacity(), capacity);
    }
}

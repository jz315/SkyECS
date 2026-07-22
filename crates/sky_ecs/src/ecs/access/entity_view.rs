use crate::ecs::{
    resolve_column_ptr, EntityId, PreparedCache, QueryDescriptor, QuerySpec, ReadOnlyQuerySpec,
    World,
};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

pub(crate) struct EntityViewCache<Q> {
    descriptor: QueryDescriptor,
    prepared: PreparedCache,
    matched_routes: Vec<u8>,
    component_ptrs: Vec<*mut u8>,
    query_width: usize,
    marker: PhantomData<fn() -> Q>,
}

impl<Q: QuerySpec> Default for EntityViewCache<Q> {
    fn default() -> Self {
        let descriptor = Q::descriptor();
        let query_width = descriptor.components.len();
        Self {
            descriptor,
            prepared: PreparedCache::default(),
            matched_routes: Vec::new(),
            component_ptrs: Vec::new(),
            query_width,
            marker: PhantomData,
        }
    }
}

impl<Q: QuerySpec> EntityViewCache<Q> {
    #[inline]
    pub(crate) fn prepare(&mut self, world: &World) {
        self.prepared.prepare::<()>(world, &self.descriptor);

        let route_slots = world.chunk_route_slot_count();
        self.matched_routes.resize(route_slots, 0);
        self.matched_routes.fill(0);

        let pointer_slots = route_slots
            .checked_mul(self.query_width)
            .expect("entity-view route table size overflow");
        self.component_ptrs.resize(pointer_slots, ptr::null_mut());

        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];
            for (chunk_index, chunk) in data.chunks.iter().enumerate() {
                let route_index = data.chunk_id(chunk_index).index();
                let offset = route_index * self.query_width;
                let pointers = &mut self.component_ptrs[offset..offset + self.query_width];

                for (pointer, &component_index) in
                    pointers.iter_mut().zip(cached.component_indices.iter())
                {
                    *pointer = resolve_column_ptr(chunk, component_index);
                }

                // Publish the match only after all route-major slots have been
                // overwritten, including null optional-component sentinels.
                self.matched_routes[route_index] = 1;
            }
        }
    }

    #[inline(always)]
    pub(crate) fn row<'a>(
        &'a self,
        world: &World,
        entity: EntityId,
    ) -> Option<(&'a [*mut u8], usize)> {
        let route = world.entity_route(entity)?;
        let route_index = route.chunk_id.index();
        if unsafe {
            // SAFETY: prepare sizes matched_routes to this World's complete
            // chunk-route slot count, and a live EntityRoute names one of
            // those slots while structure remains frozen.
            *self.matched_routes.get_unchecked(route_index)
        } == 0
        {
            return None;
        }

        let offset = route_index * self.query_width;
        let pointers = unsafe {
            // SAFETY: component_ptrs is prepared as route_slots * query_width.
            // The validated route index therefore owns this complete
            // route-major query-width range.
            self.component_ptrs
                .get_unchecked(offset..offset + self.query_width)
        };
        Some((pointers, route.entity_index))
    }
}

/// A reusable tuple-capable component view for arbitrary entity IDs.
///
/// Binding refreshes the route table from the current world while retaining
/// its allocations. Each lookup validates the entity generation and resolves
/// all query components from one entity route.
#[must_use = "prepared entity views do nothing until they are bound"]
pub struct PreparedEntityView<Q> {
    cache: EntityViewCache<Q>,
}

impl<Q: QuerySpec> Default for PreparedEntityView<Q> {
    fn default() -> Self {
        Self {
            cache: EntityViewCache::default(),
        }
    }
}

impl<Q: QuerySpec> PreparedEntityView<Q> {
    /// Creates an empty reusable entity view.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds this plan exclusively to `world`.
    ///
    /// Every bind refreshes component bases. This is required because a chunk
    /// may retain its stable ID while promotion replaces its backing block.
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
            world: NonNull::from(world),
            cache: &self.cache,
            world_marker: PhantomData,
        }
    }
}

impl<Q: ReadOnlyQuerySpec> PreparedEntityView<Q> {
    /// Binds this plan immutably to `world`.
    #[inline]
    pub fn bind<'s, 'w>(&'s mut self, world: &'w World) -> BoundEntityView<'s, 'w, Q> {
        self.cache.prepare(world);
        BoundEntityView {
            world,
            cache: &self.cache,
        }
    }
}

/// A read-only prepared entity view bound to one world.
#[must_use = "bound entity views do nothing until get is called"]
pub struct BoundEntityView<'s, 'w, Q> {
    world: &'w World,
    cache: &'s EntityViewCache<Q>,
}

impl<Q: ReadOnlyQuerySpec> BoundEntityView<'_, '_, Q> {
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
        let (pointers, entity_index) = self.cache.row(self.world, entity)?;
        Some(unsafe {
            // SAFETY: prepare wrote the descriptor-matched columns for this
            // live route. The shared bound World prevents pointer relocation
            // and Q is read-only, so all references may share this borrow.
            Q::item_from_raw_parts(pointers, entity_index)
        })
    }
}

/// An exclusive prepared entity view bound to one world.
#[must_use = "bound mutable entity views do nothing until get_mut is called"]
pub struct BoundEntityViewMut<'s, 'w, Q> {
    world: NonNull<World>,
    cache: &'s EntityViewCache<Q>,
    world_marker: PhantomData<&'w mut World>,
}

impl<Q: QuerySpec> BoundEntityViewMut<'_, '_, Q> {
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
        let world = unsafe {
            // SAFETY: world_marker retains the exclusive World borrow. This
            // temporary shared access is used only to copy route metadata and
            // ends before any component reference is constructed.
            self.world.as_ref()
        };
        let (pointers, entity_index) = self.cache.row(world, entity)?;
        Some(unsafe {
            // SAFETY: bind_mut exclusively borrows the World, prepare wrote
            // every descriptor-matched pointer, and this method's mutable
            // borrow prevents overlapping query items from the same view.
            Q::item_from_raw_parts(pointers, entity_index)
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
    fn route_tables_reuse_allocations_across_binds() {
        let mut world = World::new();
        let entity = world.spawn((Position(1), Velocity(2)));
        let mut prepared = PreparedEntityView::<(&Position, &Velocity)>::new();

        assert_eq!(prepared.bind(&world).get(entity).unwrap().0 .0, 1);
        let matched_ptr = prepared.cache.matched_routes.as_ptr();
        let matched_capacity = prepared.cache.matched_routes.capacity();
        let component_ptr = prepared.cache.component_ptrs.as_ptr();
        let component_capacity = prepared.cache.component_ptrs.capacity();

        assert_eq!(prepared.bind(&world).get(entity).unwrap().1 .0, 2);
        assert_eq!(prepared.cache.matched_routes.as_ptr(), matched_ptr);
        assert_eq!(prepared.cache.matched_routes.capacity(), matched_capacity);
        assert_eq!(prepared.cache.component_ptrs.as_ptr(), component_ptr);
        assert_eq!(prepared.cache.component_ptrs.capacity(), component_capacity);
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
        let component_ptr = prepared.cache.component_ptrs.as_ptr();
        let component_capacity = prepared.cache.component_ptrs.capacity();

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
        assert_eq!(prepared.cache.component_ptrs.as_ptr(), component_ptr);
        assert_eq!(prepared.cache.component_ptrs.capacity(), component_capacity);
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

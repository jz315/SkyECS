use crate::ecs::entity::EntityRoute;
use crate::ecs::{EntityFetchSpec, PreparedCache, QueryDescriptor, World};
use core::mem::MaybeUninit;
use std::sync::Arc;

/// Diagnostic counters for a prepared entity-view route cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntityViewCacheStats {
    /// Number of times the route cache has been rebuilt.
    pub rebuild_count: u64,
    /// Number of chunk-route slots represented by the cache.
    pub route_slots: usize,
    /// Number of typed fetch entries represented by the cache.
    pub fetch_slots: usize,
}

pub(crate) struct EntityViewCache<Q: EntityFetchSpec> {
    descriptor: QueryDescriptor,
    prepared: PreparedCache,
    pub(super) matched_routes: Vec<u8>,
    pub(super) fetches: Vec<MaybeUninit<Q::Fetch>>,
    cached_world: Option<Arc<()>>,
    cached_column_base_epoch: Option<u64>,
    rebuild_count: u64,
}

impl<Q: EntityFetchSpec> Default for EntityViewCache<Q> {
    fn default() -> Self {
        Self {
            descriptor: Q::descriptor(),
            prepared: PreparedCache::default(),
            matched_routes: Vec::new(),
            fetches: Vec::new(),
            cached_world: None,
            cached_column_base_epoch: None,
            rebuild_count: 0,
        }
    }
}

impl<Q: EntityFetchSpec> EntityViewCache<Q> {
    #[inline]
    pub(crate) fn prepare(&mut self, world: &World) {
        let same_world = self
            .cached_world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        if same_world && self.cached_column_base_epoch == Some(world.column_base_epoch()) {
            return;
        }
        self.prepared.prepare::<()>(world, &self.descriptor);

        let route_slots = world.chunk_route_slot_count();
        self.matched_routes.resize(route_slots, 0);
        self.matched_routes.fill(0);
        self.fetches.resize_with(route_slots, MaybeUninit::uninit);

        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];
            for (chunk_index, chunk) in data.chunks.iter().enumerate() {
                let route_index = data.chunk_id(chunk_index).index();
                let fetch = unsafe {
                    // SAFETY: PreparedCache produced this descriptor-matched
                    // component map for this live chunk.
                    Q::prepare_fetch(chunk, &cached.component_indices)
                };
                self.fetches[route_index].write(fetch);

                // Publish the match only after the complete typed fetch has
                // been initialized, including nullable optional bases.
                self.matched_routes[route_index] = 1;
            }
        }
        self.cached_world = Some(Arc::clone(world.cache_token()));
        self.cached_column_base_epoch = Some(world.column_base_epoch());
        self.rebuild_count = self
            .rebuild_count
            .checked_add(1)
            .expect("entity-view rebuild counter exhausted");
    }

    pub(crate) fn stats(&self) -> EntityViewCacheStats {
        EntityViewCacheStats {
            rebuild_count: self.rebuild_count,
            route_slots: self.matched_routes.len(),
            fetch_slots: self.fetches.len(),
        }
    }

    #[inline(always)]
    pub(crate) fn row(&self, route: EntityRoute) -> Option<(&Q::Fetch, usize)> {
        let route_index = route.chunk_id.index();
        if unsafe {
            // SAFETY: prepare sizes matched_routes to this World's complete
            // route table, and a live EntityRoute names one of those slots
            // while structure remains frozen.
            *self.matched_routes.get_unchecked(route_index)
        } == 0
        {
            return None;
        }

        let fetch = unsafe {
            // SAFETY: a nonzero matched marker is published only after this
            // route's typed fetch has been initialized. No structural change
            // can invalidate it while the bound view remains live.
            self.fetches.get_unchecked(route_index).assume_init_ref()
        };
        Some((fetch, route.entity_index))
    }
}

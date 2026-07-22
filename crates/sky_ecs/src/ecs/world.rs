#![deny(unsafe_op_in_unsafe_fn)]

use super::commands::{PendingComponentCommand, PendingComponentEntry};
use super::component_posting::{ComponentPostingIndex, ComponentPostingList};
use super::resource::Resources;
use super::*;
use crate::ecs::entity::{EntityLocation, EntityRecord, EntityRoute};
use crate::ecs::system::{Schedule, StageBuilder};
use crate::ecs::time::Time;
use crate::ecs::{component_type, ComponentType};
use crate::plugin::{Plugin, PluginError, PluginRegistry, PluginResult};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::ptr::NonNull;
use std::sync::Arc;

mod columns;
mod entities;
mod epochs;
mod queries;
mod resources;
mod schedule;
mod transitions;

use epochs::{ChunkSetEpochGuard, StorageEpochs};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TransitionKey {
    archetype: Archetype,
    component_id: usize,
    add: bool,
}

#[derive(Clone, Copy)]
struct CopySpan {
    source_component: u8,
    target_component: u8,
    component_size: u32,
}

struct TransitionPlan {
    copy_spans: SmallVec<[CopySpan; 8]>,
    target_component_index: Option<u8>,
    target_data_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ComponentCommandTransitionKey {
    archetype: Archetype,
    operations: SmallVec<[(usize, bool); 4]>,
}

struct ComponentCommandTransitionPlan {
    target_archetype: Archetype,
    target_data_index: usize,
    copy_spans: SmallVec<[CopySpan; 8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldPoisonReason {
    CommandPanic,
    SchedulePanic,
}

impl WorldPoisonReason {
    fn description(self) -> &'static str {
        match self {
            Self::CommandPanic => "a deferred command panic",
            Self::SchedulePanic => "a schedule or system panic",
        }
    }
}

/// The central container for all ECS data.
///
/// A `World` owns every entity, component, and resource.  It provides methods
/// for spawning and despawning entities, reading and writing components,
/// scheduling systems, and running queries.
///
/// # Drop Semantics
///
/// When a `World` is dropped, all component destructors are called
/// automatically.  Non-`Copy` components (e.g. `String`, `Vec<T>`) are
/// handled correctly.
///
/// # Examples
///
/// ```
/// use sky_ecs::World;
///
/// #[derive(Clone, Copy)]
/// struct Position { x: f32, y: f32 }
///
/// let mut world = World::new();
/// let entity = world.spawn((Position { x: 0.0, y: 0.0 },));
/// assert_eq!(world.get::<Position>(entity).unwrap().x, 0.0);
/// ```
pub struct World {
    cache_token: Arc<()>,
    query_cache: QueryCacheStore,
    poison_reason: Option<WorldPoisonReason>,
    pub time: Time,
    pub(crate) data: Vec<ArchetypeStorage>,
    archetype_epoch: usize,
    storage_epochs: StorageEpochs,
    resource_epoch: u64,
    archetype_to_data_index: FxHashMap<Archetype, usize>,
    component_postings: ComponentPostingIndex,
    last_data_index: Option<(Archetype, usize)>,
    transitions: FxHashMap<TransitionKey, Box<TransitionPlan>>,
    last_transition: Option<(TransitionKey, NonNull<TransitionPlan>)>,
    component_command_transitions:
        FxHashMap<ComponentCommandTransitionKey, Box<ComponentCommandTransitionPlan>>,
    chunk_directory: ChunkDirectory,
    entities: Vec<EntityRecord>,
    free_entities: Vec<u32>,
    live_entity_count: usize,
    resources: Resources,
    plugins: PluginRegistry,
    schedule: Option<Schedule>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates an empty world with no entities, resources, or systems.
    pub fn new() -> Self {
        Self {
            cache_token: Arc::new(()),
            query_cache: QueryCacheStore::default(),
            poison_reason: None,
            time: Time::default(),
            data: Vec::new(),
            archetype_epoch: 0,
            storage_epochs: StorageEpochs::default(),
            resource_epoch: 0,
            archetype_to_data_index: FxHashMap::default(),
            component_postings: ComponentPostingIndex::default(),
            last_data_index: None,
            transitions: FxHashMap::default(),
            last_transition: None,
            component_command_transitions: FxHashMap::default(),
            chunk_directory: ChunkDirectory::default(),
            entities: Vec::new(),
            free_entities: Vec::new(),
            live_entity_count: 0,
            resources: Resources::default(),
            plugins: PluginRegistry::default(),
            schedule: Some(Schedule::default()),
        }
    }

    /// Returns whether a command, schedule, or system panicked while mutating
    /// this World.
    ///
    /// A poisoned World remains inspectable and can be shut down, but it
    /// cannot safely apply more command buffers or advance its schedule: an
    /// command or system may have mutated only part of its intended state.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poison_reason.is_some()
    }

    fn allocate_entity(&mut self) -> EntityId {
        if let Some(index) = self.free_entities.pop() {
            let record = &self.entities[index as usize];
            EntityId::new(index, record.generation)
        } else {
            assert!(
                self.entities.len() < u32::MAX as usize,
                "entity slot limit exhausted"
            );
            let index = self.entities.len() as u32;
            self.entities.push(EntityRecord::vacant(0));
            EntityId::new(index, 0)
        }
    }

    #[inline(always)]
    fn allocate_entity_at_location(
        entities: &mut Vec<EntityRecord>,
        free_entities: &mut Vec<u32>,
        route: EntityRoute,
    ) -> EntityId {
        if let Some(index) = free_entities.pop() {
            let record = &mut entities[index as usize];
            let entity = EntityId::new(index, record.generation);
            record.set_route(route);
            entity
        } else {
            assert!(
                entities.len() < u32::MAX as usize,
                "entity slot limit exhausted"
            );
            let index = entities.len() as u32;
            entities.push(EntityRecord::occupied(0, route));
            EntityId::new(index, 0)
        }
    }

    #[inline(always)]
    fn ensure_chunk_route(&mut self, data_index: usize, chunk_index: usize) -> ChunkId {
        let directory = &mut self.chunk_directory;
        let storage = &mut self.data[data_index];
        directory.ensure(&mut storage.chunk_ids[chunk_index], data_index, chunk_index)
    }

    /// Allocates one uninitialized storage row and registers its physical
    /// chunk before an entity record can make that row observable.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component in the returned row before
    /// exposing the entity or allowing that row to be dropped.
    #[inline(always)]
    unsafe fn allocate_storage_row(
        &mut self,
        data_index: usize,
        entity: EntityId,
    ) -> ChunkEntityLocation {
        let location = {
            let mut storage =
                ChunkSetEpochGuard::new(&mut self.data[data_index], &mut self.storage_epochs);
            unsafe { storage.storage_mut().add_entity(entity) }
        };
        self.ensure_chunk_route(data_index, location.chunk_index);
        location
    }

    #[inline(always)]
    fn remove_storage_row(
        &mut self,
        data_index: usize,
        location: ChunkEntityLocation,
    ) -> ChunkRemoval {
        let mut storage =
            ChunkSetEpochGuard::new(&mut self.data[data_index], &mut self.storage_epochs);
        storage.storage_mut().remove_entity(location)
    }

    #[inline(always)]
    fn finish_chunk_removal(&mut self, data_index: usize, removal: ChunkRemoval) {
        if let Some((moved_entity, moved_location)) = removal.moved {
            self.set_entity_location(
                moved_entity,
                EntityLocation {
                    data_index,
                    chunk_index: moved_location.chunk_index,
                    entity_index: moved_location.entity_index,
                },
            );
        }
        if let Some(retired_chunk) = removal.retired_chunk {
            self.chunk_directory.release(retired_chunk);
        }
    }

    /// Invalidates cached resource views after the resource set changes.
    #[inline(always)]
    fn bump_resource_epoch(&mut self) {
        self.resource_epoch = self
            .resource_epoch
            .checked_add(1)
            .expect("world resource epoch exhausted");
    }

    #[inline(always)]
    fn bump_archetype_epoch(&mut self) {
        self.archetype_epoch = self
            .archetype_epoch
            .checked_add(1)
            .expect("world archetype epoch exhausted");
    }

    pub(crate) fn assert_command_apply_allowed(&self) {
        if let Some(reason) = self.poison_reason {
            panic!(
                "cannot apply commands to a poisoned World after {}",
                reason.description()
            );
        }
    }

    pub(crate) fn poison_after_command_panic(&mut self) {
        if self.poison_reason.is_none() {
            self.poison_reason = Some(WorldPoisonReason::CommandPanic);
        }
    }

    pub(crate) fn poison_after_schedule_panic(&mut self) {
        if self.poison_reason.is_none() {
            self.poison_reason = Some(WorldPoisonReason::SchedulePanic);
        }
    }

    fn assert_schedule_tick_allowed(&self) {
        if let Some(reason) = self.poison_reason {
            panic!(
                "cannot tick a poisoned World after {}",
                reason.description()
            );
        }
    }

    #[inline(always)]
    fn ensure_data_index(&mut self, archetype: Archetype) -> usize {
        if let Some((cached_archetype, data_index)) = self.last_data_index {
            if cached_archetype == archetype {
                return data_index;
            }
        }

        if let Some(index) = self.archetype_to_data_index.get(&archetype).copied() {
            self.last_data_index = Some((archetype, index));
            return index;
        }

        self.bump_archetype_epoch();
        let index = self.data.len();
        self.data.push(ArchetypeStorage::new(archetype));
        self.component_postings.append_archetype(index, &archetype);
        self.archetype_to_data_index.insert(archetype, index);
        self.last_data_index = Some((archetype, index));
        index
    }

    #[inline(always)]
    pub(crate) fn component_posting(
        &self,
        component: &ComponentType,
    ) -> Option<&ComponentPostingList> {
        self.component_postings.list(component)
    }

    #[inline(always)]
    pub(crate) fn entity_location(&self, entity: EntityId) -> Option<EntityLocation> {
        let route = self.entity_route(entity)?;
        let address = self
            .chunk_directory
            .resolve(route.chunk_id)
            .expect("live entity must reference a registered chunk");
        Some(EntityLocation {
            data_index: address.data_index,
            chunk_index: address.chunk_index,
            entity_index: route.entity_index,
        })
    }

    #[inline(always)]
    pub(crate) fn entity_route(&self, entity: EntityId) -> Option<EntityRoute> {
        let record = self.entities.get(entity.index() as usize)?;
        if record.generation != entity.generation() {
            return None;
        }

        record.route()
    }

    #[inline(always)]
    pub(crate) fn set_entity_location(&mut self, entity: EntityId, location: EntityLocation) {
        let chunk_id = self.data[location.data_index].chunk_id(location.chunk_index);
        assert!(
            chunk_id.is_assigned(),
            "entity row must belong to a registered chunk"
        );
        let record = &mut self.entities[entity.index() as usize];
        debug_assert_eq!(record.generation, entity.generation());
        record.set_route(EntityRoute {
            chunk_id,
            entity_index: location.entity_index,
        });
    }

    #[inline(always)]
    pub(crate) fn chunk_route_slot_count(&self) -> usize {
        self.chunk_directory.slot_count()
    }

    /// Returns the total number of live entities across all archetypes.
    pub fn entity_count(&self) -> usize {
        self.live_entity_count
    }

    /// Returns the number of distinct archetypes currently stored.
    pub fn archetype_count(&self) -> usize {
        self.data.len()
    }

    /// Returns diagnostic counts for the World-local chunk route table.
    pub fn route_table_stats(&self) -> RouteTableStats {
        self.chunk_directory.stats()
    }

    /// Releases trailing vacant route slots without renumbering live chunks.
    ///
    /// Internal holes remain available for reuse. This operation is useful
    /// after a temporary loading peak whose chunks occupied the tail of the
    /// route table.
    pub fn shrink_route_tables(&mut self) -> RouteTableStats {
        if self.chunk_directory.shrink_tail() {
            self.bump_column_base_epoch();
        }
        self.chunk_directory.stats()
    }

    /// Removes all entities and their components, but keeps resources.
    ///
    /// Component destructors are called for every live entity.
    /// If one or more destructors panic, all entities are still removed before
    /// the first panic resumes.
    pub fn clear(&mut self) {
        self.bump_row_layout_epoch();
        self.bump_chunk_set_epoch();
        self.bump_column_base_epoch();
        self.bump_active_storage_epoch();
        self.bump_archetype_epoch();
        let mut drop_panic = None;

        // Drop every component under an unwind boundary, then mark each chunk
        // empty before destroying its backing storage. This makes `clear`
        // transactional with respect to ownership: even when a user Drop
        // implementation panics, no live row remains visible and no value is
        // dropped a second time while `self.data` is cleared.
        for data in &mut self.data {
            for chunk in &mut data.chunks {
                let entity_count = chunk.entity_count;
                for entity_index in 0..entity_count {
                    // Safety: this loop visits each live droppable value
                    // exactly once. `entity_count` is reset before the chunk
                    // is destroyed, including after captured panics.
                    unsafe {
                        Self::drop_entity_components_catching(chunk, entity_index, &mut drop_panic);
                    }
                }
                chunk.entity_count = 0;
            }
        }
        self.data.clear();
        self.archetype_to_data_index.clear();
        self.component_postings.clear();
        self.last_data_index = None;
        self.transitions.clear();
        self.last_transition = None;
        self.component_command_transitions.clear();
        self.chunk_directory.clear();
        self.free_entities.clear();
        for (index, record) in self.entities.iter_mut().enumerate() {
            if record.is_alive() {
                record.clear_route();
                if let Some(next_generation) = record.generation.checked_add(1) {
                    record.generation = next_generation;
                }
            }
            if record.generation != u32::MAX {
                self.free_entities.push(index as u32);
            }
        }
        self.live_entity_count = 0;

        if let Some(payload) = drop_panic {
            std::panic::resume_unwind(payload);
        }
    }
}

#[cfg(test)]
mod tests;

#![deny(unsafe_op_in_unsafe_fn)]

use super::commands::{PendingComponentCommand, PendingComponentEntry};
use super::component_posting::{ComponentPostingIndex, ComponentPostingList};
use super::resource::Resources;
use super::*;
use crate::ecs::entity::{EntityLocation, EntityRecord};
use crate::ecs::system::{Schedule, StageBuilder};
use crate::ecs::time::Time;
use crate::ecs::{component_type, ComponentType};
use crate::plugin::{Plugin, PluginError, PluginRegistry, PluginResult};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::ptr::NonNull;
use std::sync::Arc;

mod entities;
mod queries;
mod resources;
mod schedule;
mod transitions;

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
    command_poisoned: bool,
    pub time: Time,
    pub(crate) data: Vec<ArchetypeStorage>,
    archetype_epoch: usize,
    storage_epoch: u64,
    resource_epoch: u64,
    archetype_to_data_index: FxHashMap<Archetype, usize>,
    component_postings: ComponentPostingIndex,
    last_data_index: Option<(Archetype, usize)>,
    transitions: FxHashMap<TransitionKey, Box<TransitionPlan>>,
    last_transition: Option<(TransitionKey, NonNull<TransitionPlan>)>,
    component_command_transitions:
        FxHashMap<ComponentCommandTransitionKey, Box<ComponentCommandTransitionPlan>>,
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
            command_poisoned: false,
            time: Time::default(),
            data: Vec::new(),
            archetype_epoch: 0,
            storage_epoch: 0,
            resource_epoch: 0,
            archetype_to_data_index: FxHashMap::default(),
            component_postings: ComponentPostingIndex::default(),
            last_data_index: None,
            transitions: FxHashMap::default(),
            last_transition: None,
            component_command_transitions: FxHashMap::default(),
            entities: Vec::new(),
            free_entities: Vec::new(),
            live_entity_count: 0,
            resources: Resources::default(),
            plugins: PluginRegistry::default(),
            schedule: Some(Schedule::default()),
        }
    }

    /// Returns whether a deferred command panicked while being applied.
    ///
    /// A poisoned World remains inspectable and can be shut down, but it
    /// cannot safely apply more command buffers or advance its schedule: an
    /// arbitrary command may have mutated only part of its intended state.
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.command_poisoned
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
        location: EntityLocation,
    ) -> EntityId {
        if let Some(index) = free_entities.pop() {
            let record = &mut entities[index as usize];
            let entity = EntityId::new(index, record.generation);
            record.set_location(location);
            entity
        } else {
            assert!(
                entities.len() < u32::MAX as usize,
                "entity slot limit exhausted"
            );
            let index = entities.len() as u32;
            entities.push(EntityRecord::occupied(0, location));
            EntityId::new(index, 0)
        }
    }

    /// Invalidates cached views whose raw ranges depend on chunk layout.
    ///
    /// Bump this before any operation that can change chunk addresses, entity
    /// ranges, or entity ordering. Component value updates do not affect it.
    #[inline(always)]
    fn bump_storage_epoch(&mut self) {
        self.storage_epoch = self
            .storage_epoch
            .checked_add(1)
            .expect("world storage epoch exhausted");
    }

    #[inline(always)]
    fn bump_resource_epoch(&mut self) {
        self.resource_epoch = self
            .resource_epoch
            .checked_add(1)
            .expect("world resource epoch exhausted");
    }

    pub(crate) fn assert_command_apply_allowed(&self) {
        assert!(
            !self.command_poisoned,
            "cannot apply commands to a poisoned World"
        );
    }

    pub(crate) fn poison_after_command_panic(&mut self) {
        self.command_poisoned = true;
    }

    fn assert_schedule_tick_allowed(&self) {
        assert!(
            !self.command_poisoned,
            "cannot tick a poisoned World after a deferred command panic"
        );
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

        let index = self.data.len();
        self.data.push(ArchetypeStorage::new(archetype));
        self.component_postings.append_archetype(index, &archetype);
        self.archetype_to_data_index.insert(archetype, index);
        self.last_data_index = Some((archetype, index));
        self.archetype_epoch += 1;
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
        let record = self.entities.get(entity.index() as usize)?;
        if record.generation != entity.generation() {
            return None;
        }

        record.location()
    }

    #[inline(always)]
    pub(crate) fn set_entity_location(&mut self, entity: EntityId, location: EntityLocation) {
        let record = &mut self.entities[entity.index() as usize];
        debug_assert_eq!(record.generation, entity.generation());
        record.set_location(location);
    }

    /// Returns the total number of live entities across all archetypes.
    pub fn entity_count(&self) -> usize {
        self.live_entity_count
    }

    /// Returns the number of distinct archetypes currently stored.
    pub fn archetype_count(&self) -> usize {
        self.data.len()
    }

    /// Removes all entities and their components, but keeps resources.
    ///
    /// Component destructors are called for every live entity.
    /// If one or more destructors panic, all entities are still removed before
    /// the first panic resumes.
    pub fn clear(&mut self) {
        self.bump_storage_epoch();
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
        self.free_entities.clear();
        for (index, record) in self.entities.iter_mut().enumerate() {
            if record.is_alive() {
                record.clear_location();
                if let Some(next_generation) = record.generation.checked_add(1) {
                    record.generation = next_generation;
                }
            }
            if record.generation != u32::MAX {
                self.free_entities.push(index as u32);
            }
        }
        self.live_entity_count = 0;
        self.archetype_epoch += 1;

        if let Some(payload) = drop_panic {
            std::panic::resume_unwind(payload);
        }
    }
}

#[cfg(test)]
mod tests;

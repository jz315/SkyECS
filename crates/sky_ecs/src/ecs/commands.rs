#![deny(unsafe_op_in_unsafe_fn)]

use super::erased_value::InsertValue;
use super::{Bundle, EntityId, World};
use crate::ecs::{component_type, ComponentType};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::any::Any;
use std::mem;

// ---------------------------------------------------------------------------
// Deferred command trait – closures applied to &mut World
// ---------------------------------------------------------------------------

trait DeferredWorldCommand {
    fn apply(self: Box<Self>, world: &mut World);
}

struct FnDeferredCommand<F>(F);

impl<F> DeferredWorldCommand for FnDeferredCommand<F>
where
    F: FnOnce(&mut World) + 'static,
{
    fn apply(self: Box<Self>, world: &mut World) {
        (self.0)(world);
    }
}

// ---------------------------------------------------------------------------
// Spawn-batch coalescing
// ---------------------------------------------------------------------------

trait SpawnBatchCommand {
    fn apply(&mut self, world: &mut World);
    fn clear(&mut self);
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct TypedSpawnBatch<B> {
    bundles: Vec<B>,
}

impl<B> SpawnBatchCommand for TypedSpawnBatch<B>
where
    B: Bundle,
{
    fn apply(&mut self, world: &mut World) {
        world.spawn_batch(self.bundles.drain(..));
    }

    fn clear(&mut self) {
        self.bundles.clear();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Command queue
// ---------------------------------------------------------------------------

enum Command {
    Vacant,
    EntityBatch(PendingEntityBuffer),
    SpawnBatch(Box<dyn SpawnBatchCommand>),
    Deferred(Box<dyn DeferredWorldCommand>),
}

impl Command {
    fn apply(self, world: &mut World) -> Self {
        match self {
            Self::Vacant => Self::Vacant,
            Self::EntityBatch(mut batch) => {
                batch.flush(world);
                Self::EntityBatch(batch)
            }
            Self::SpawnBatch(mut batch) => {
                batch.apply(world);
                Self::SpawnBatch(batch)
            }
            Self::Deferred(command) => {
                command.apply(world);
                Self::Vacant
            }
        }
    }

    fn clear(self) -> Self {
        match self {
            Self::Vacant => Self::Vacant,
            Self::EntityBatch(mut batch) => {
                batch.clear();
                Self::EntityBatch(batch)
            }
            Self::SpawnBatch(mut batch) => {
                batch.clear();
                Self::SpawnBatch(batch)
            }
            Self::Deferred(command) => {
                drop(command);
                Self::Vacant
            }
        }
    }
}

enum EntityCommand {
    Despawn(EntityId),
    Insert {
        entity: EntityId,
        component: ComponentType,
        value: InsertValue,
    },
    Remove {
        entity: EntityId,
        component: ComponentType,
    },
}

// ---------------------------------------------------------------------------
// Per-entity coalesced command state
// ---------------------------------------------------------------------------

pub(super) enum PendingComponentCommand {
    Insert(InsertValue),
    Remove,
}

pub(super) struct PendingComponentEntry {
    pub(super) component: ComponentType,
    pub(super) command: PendingComponentCommand,
}

#[derive(Default)]
struct PendingEntityCommands {
    despawn: bool,
    components: SmallVec<[PendingComponentEntry; 4]>,
}

impl PendingEntityCommands {
    fn queue_insert(&mut self, component: ComponentType, value: InsertValue) {
        if self.despawn {
            return;
        }

        if let Some(pending) = self
            .components
            .iter_mut()
            .find(|p| p.component.id() == component.id())
        {
            pending.command = PendingComponentCommand::Insert(value);
            return;
        }

        self.components.push(PendingComponentEntry {
            component,
            command: PendingComponentCommand::Insert(value),
        });
    }

    fn queue_remove(&mut self, component: ComponentType) {
        if self.despawn {
            return;
        }

        if let Some(pending) = self
            .components
            .iter_mut()
            .find(|p| p.component.id() == component.id())
        {
            pending.command = PendingComponentCommand::Remove;
            return;
        }

        self.components.push(PendingComponentEntry {
            component,
            command: PendingComponentCommand::Remove,
        });
    }

    fn queue_despawn(&mut self) {
        self.despawn = true;
        self.components.clear();
    }
}

// ---------------------------------------------------------------------------
// PendingEntityBuffer – Vec-backed coalesced entity command buffer
//
// Primary storage is `entries`, preserving first-seen entity order.
// `index` is a dedup-only map from EntityId to position in `entries`.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct PendingEntityBuffer {
    entries: Vec<(EntityId, PendingEntityCommands)>,
    index: FxHashMap<EntityId, u32>,
}

impl PendingEntityBuffer {
    fn push(&mut self, command: EntityCommand) {
        let entity = match &command {
            EntityCommand::Despawn(entity) => *entity,
            EntityCommand::Insert { entity, .. } => *entity,
            EntityCommand::Remove { entity, .. } => *entity,
        };

        let pending = if let Some(&pos) = self.index.get(&entity) {
            &mut self.entries[pos as usize].1
        } else {
            let pos = self.entries.len();
            debug_assert!(
                pos <= u32::MAX as usize,
                "PendingEntityBuffer: entry count exceeds u32 index capacity"
            );
            self.index.insert(entity, pos as u32);
            self.entries
                .push((entity, PendingEntityCommands::default()));
            &mut self.entries.last_mut().unwrap().1
        };

        match command {
            EntityCommand::Despawn(_) => pending.queue_despawn(),
            EntityCommand::Insert {
                component, value, ..
            } => pending.queue_insert(component, value),
            EntityCommand::Remove { component, .. } => pending.queue_remove(component),
        }
    }

    fn flush(&mut self, world: &mut World) {
        // The map is only needed while recording. Clear it before any user
        // destructor can unwind so the buffer is observably empty on panic,
        // while retaining its allocation for the next frame.
        self.index.clear();
        for (entity, pending) in self.entries.drain(..) {
            if pending.despawn {
                world.despawn(entity);
                continue;
            }

            let mut components = pending.components;
            world.apply_component_commands(entity, &mut components);
        }
    }

    fn clear(&mut self) {
        self.index.clear();
        self.entries.clear();
    }
}

// ---------------------------------------------------------------------------
// Owned command buffer
// ---------------------------------------------------------------------------

/// A deferred command buffer for batching structural ECS changes.
///
/// Commands are recorded and then applied in one explicit pass via
/// [`apply`](Self::apply).
/// This is useful when you need to make structural changes (spawn, despawn,
/// insert, remove) from within a query loop or a system, where direct
/// mutation of the [`World`] is not possible.
///
/// Commands targeting the same entity are **coalesced** — only the final
/// state per component is applied, reducing archetype migrations.
///
/// # Examples
///
/// ```
/// # use sky_ecs::{CommandBuffer, World};
/// # #[derive(Clone, Copy)] struct Health(f32);
/// # let mut world = World::new();
/// let entity = world.spawn((Health(100.0),));
///
/// let mut cmds = CommandBuffer::new();
/// cmds.insert(entity, Health(50.0));
/// cmds.apply(&mut world);
///
/// assert_eq!(world.get::<Health>(entity).unwrap().0, 50.0);
/// ```
#[derive(Default)]
pub struct CommandBuffer {
    queue: Vec<Command>,
    active_commands: usize,
    queued_count: usize,
}

/// Temporarily owns the active command prefix while preserving reusable slots.
///
/// Each command is replaced with `Vacant` before it can run user code. If that
/// code unwinds, `Drop` discards the unvisited commands and the public buffer
/// has already been restored to its empty invariant.
struct ActiveCommands<'a> {
    slots: &'a mut [Command],
    next: usize,
}

impl<'a> ActiveCommands<'a> {
    fn new(slots: &'a mut [Command]) -> Self {
        Self { slots, next: 0 }
    }

    fn next(&mut self) -> Option<(usize, Command)> {
        if self.next == self.slots.len() {
            return None;
        }
        let index = self.next;
        self.next += 1;
        Some((index, mem::replace(&mut self.slots[index], Command::Vacant)))
    }

    fn restore(&mut self, index: usize, command: Command) {
        debug_assert!(matches!(self.slots[index], Command::Vacant));
        self.slots[index] = command;
    }
}

impl Drop for ActiveCommands<'_> {
    fn drop(&mut self) {
        let already_panicking = std::thread::panicking();
        let mut first_cleanup_panic = None;

        for slot in &mut self.slots[self.next..] {
            let command = mem::replace(slot, Command::Vacant);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drop(command);
            }));

            if let Err(payload) = result {
                if already_panicking || first_cleanup_panic.is_some() {
                    // Cleanup must not replace an in-flight panic or trigger a
                    // double-panic abort. Most panic payloads can still be
                    // released normally; only an adversarial payload whose
                    // own destructor panics has to be leaked as a last resort.
                    drop_panic_payload_without_unwinding(payload);
                } else {
                    first_cleanup_panic = Some(payload);
                }
            }
        }

        if let Some(payload) = first_cleanup_panic {
            // No panic was active when cleanup began. Finish clearing every
            // slot, then propagate the first cleanup failure.
            std::panic::resume_unwind(payload);
        }
    }
}

fn drop_panic_payload_without_unwinding(payload: Box<dyn Any + Send>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(payload)));
    if let Err(nested_payload) = result {
        // A panic payload is user-owned and may itself have a panicking Drop.
        // Letting that unwind while another panic is active would abort the
        // process, so this final adversarial payload is intentionally leaked.
        mem::forget(nested_payload);
    }
}

struct CommandApplyGuard<'w> {
    world: &'w mut World,
    completed: bool,
}

impl<'w> CommandApplyGuard<'w> {
    fn new(world: &'w mut World) -> Self {
        world.assert_command_apply_allowed();
        Self {
            world,
            completed: false,
        }
    }

    #[inline(always)]
    fn world(&mut self) -> &mut World {
        self.world
    }

    fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for CommandApplyGuard<'_> {
    fn drop(&mut self) {
        if !self.completed && std::thread::panicking() {
            self.world.poison_after_command_panic();
        }
    }
}

impl CommandBuffer {
    /// Creates a new, empty command buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if no commands have been recorded.
    pub fn is_empty(&self) -> bool {
        self.queued_count == 0
    }

    /// Returns the number of commands recorded so far.
    pub fn len(&self) -> usize {
        self.queued_count
    }

    fn push_deferred<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + 'static,
    {
        self.queued_count += 1;
        self.activate(Command::Deferred(Box::new(FnDeferredCommand(f))));
    }

    fn push_entity(&mut self, command: EntityCommand) {
        self.queued_count += 1;

        if let Some(Command::EntityBatch(batch)) = self.active_last_mut() {
            batch.push(command);
            return;
        }

        let index = self.active_commands;
        self.active_commands += 1;
        if let Some(Command::EntityBatch(batch)) = self.queue.get_mut(index) {
            batch.push(command);
            return;
        }

        let mut batch = PendingEntityBuffer::default();
        batch.push(command);
        self.replace_or_push(index, Command::EntityBatch(batch));
    }

    /// Records a deferred spawn.  Consecutive spawns of the same bundle
    /// type are coalesced into a single batch for efficiency.
    pub fn spawn<B>(&mut self, bundle: B)
    where
        B: Bundle,
    {
        self.queued_count += 1;

        if let Some(Command::SpawnBatch(batch)) = self.active_last_mut() {
            if let Some(batch) = batch.as_any_mut().downcast_mut::<TypedSpawnBatch<B>>() {
                batch.bundles.push(bundle);
                return;
            }
        }

        let index = self.active_commands;
        self.active_commands += 1;
        if let Some(Command::SpawnBatch(batch)) = self.queue.get_mut(index) {
            if let Some(batch) = batch.as_any_mut().downcast_mut::<TypedSpawnBatch<B>>() {
                batch.bundles.push(bundle);
                return;
            }
        }

        self.replace_or_push(
            index,
            Command::SpawnBatch(Box::new(TypedSpawnBatch {
                bundles: vec![bundle],
            })),
        );
    }

    /// Records a deferred entity despawn.
    pub fn despawn(&mut self, entity: EntityId) {
        self.push_entity(EntityCommand::Despawn(entity));
    }

    /// Records a deferred component insertion (or overwrite).
    pub fn insert<T>(&mut self, entity: EntityId, component: T)
    where
        T: 'static,
    {
        self.push_entity(EntityCommand::Insert {
            entity,
            component: component_type::<T>(),
            value: InsertValue::from_value(component),
        });
    }

    /// Records a deferred component removal.
    pub fn remove<T>(&mut self, entity: EntityId)
    where
        T: 'static,
    {
        self.push_entity(EntityCommand::Remove {
            entity,
            component: component_type::<T>(),
        });
    }

    /// Records a deferred resource insertion.
    pub fn insert_resource<R>(&mut self, resource: R)
    where
        R: 'static,
    {
        self.push_deferred(move |world| {
            world.insert_resource(resource);
        });
    }

    /// Records a deferred resource removal.
    pub fn remove_resource<R>(&mut self)
    where
        R: 'static,
    {
        self.push_deferred(move |world| {
            world.remove_resource::<R>();
        });
    }

    /// Applies all recorded commands to the world and clears the buffer.
    ///
    /// # Panics and poisoning
    ///
    /// Commands may contain arbitrary user values whose code or destructors
    /// can panic. General rollback is therefore impossible. If a panic escapes
    /// this apply pass, the World is marked poisoned and rejects later command
    /// application and schedule ticks rather than running with a partial
    /// commit. The World remains inspectable and may be shut down.
    pub fn apply(&mut self, world: &mut World) {
        let mut apply_guard = CommandApplyGuard::new(world);
        // Restore the observable invariant before user-owned drops/deferred
        // work can unwind. The active prefix remains as reusable empty slots
        // after a successful pass.
        let active_commands = mem::take(&mut self.active_commands);
        self.queued_count = 0;
        let mut commands = ActiveCommands::new(&mut self.queue[..active_commands]);
        while let Some((index, command)) = commands.next() {
            let command = command.apply(apply_guard.world());
            commands.restore(index, command);
        }
        drop(commands);
        apply_guard.complete();
    }

    /// Discards every pending command while preserving allocated capacity.
    pub fn clear(&mut self) {
        let active_commands = mem::take(&mut self.active_commands);
        self.queued_count = 0;
        let mut commands = ActiveCommands::new(&mut self.queue[..active_commands]);
        while let Some((index, command)) = commands.next() {
            let command = command.clear();
            commands.restore(index, command);
        }
    }

    fn active_last_mut(&mut self) -> Option<&mut Command> {
        self.active_commands
            .checked_sub(1)
            .map(|index| &mut self.queue[index])
    }

    fn activate(&mut self, command: Command) {
        let index = self.active_commands;
        self.active_commands += 1;
        self.replace_or_push(index, command);
    }

    fn replace_or_push(&mut self, index: usize, command: Command) {
        if let Some(slot) = self.queue.get_mut(index) {
            *slot = command;
        } else {
            debug_assert_eq!(index, self.queue.len());
            self.queue.push(command);
        }
    }
}

// ---------------------------------------------------------------------------
// Borrowed system command writer
// ---------------------------------------------------------------------------

/// A system-local deferred structural writer.
///
/// Each scheduled system receives an isolated writer. The scheduler applies
/// the underlying buffers in deterministic registration order at the next
/// documented flush boundary.
pub struct Commands<'w> {
    buffer: &'w mut CommandBuffer,
}

impl<'w> Commands<'w> {
    /// # Safety
    ///
    /// `buffer` must be non-null, point to a live `CommandBuffer`, and be
    /// exclusively borrowed for the returned lifetime.
    pub(crate) unsafe fn from_ptr(buffer: *mut CommandBuffer) -> Self {
        Self {
            // SAFETY: the caller provides validity, alignment, and exclusive
            // access for `'w` as required by this constructor.
            buffer: unsafe { &mut *buffer },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn spawn<B>(&mut self, bundle: B)
    where
        B: Bundle + Send,
    {
        self.buffer.spawn(bundle);
    }

    pub fn despawn(&mut self, entity: EntityId) {
        self.buffer.despawn(entity);
    }

    pub fn insert<T>(&mut self, entity: EntityId, component: T)
    where
        T: Send + 'static,
    {
        self.buffer.insert(entity, component);
    }

    pub fn remove<T>(&mut self, entity: EntityId)
    where
        T: 'static,
    {
        self.buffer.remove::<T>(entity);
    }

    pub fn insert_resource<R>(&mut self, resource: R)
    where
        R: Send + 'static,
    {
        self.buffer.insert_resource(resource);
    }

    pub fn remove_resource<R>(&mut self)
    where
        R: 'static,
    {
        self.buffer.remove_resource::<R>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct Marker;

    #[test]
    fn entity_batches_preserve_first_seen_order_and_reuse_storage() {
        let mut world = World::new();
        let entities = (0..64).map(|_| world.spawn((Marker,))).collect::<Vec<_>>();
        let mut commands = CommandBuffer::new();

        commands.despawn(entities[7]);
        commands.remove::<Marker>(entities[2]);
        commands.insert(entities[7], Marker);
        commands.despawn(entities[40]);

        let (entries_capacity, index_capacity) = match &commands.queue[0] {
            Command::EntityBatch(batch) => {
                assert_eq!(
                    batch
                        .entries
                        .iter()
                        .map(|(entity, _)| *entity)
                        .collect::<Vec<_>>(),
                    vec![entities[7], entities[2], entities[40]]
                );
                (batch.entries.capacity(), batch.index.capacity())
            }
            _ => panic!("expected an entity batch"),
        };

        commands.apply(&mut world);
        assert!(commands.is_empty());
        match &commands.queue[0] {
            Command::EntityBatch(batch) => {
                assert!(batch.entries.is_empty());
                assert!(batch.index.is_empty());
                assert_eq!(batch.entries.capacity(), entries_capacity);
                assert_eq!(batch.index.capacity(), index_capacity);
            }
            _ => panic!("expected the reusable entity batch"),
        }

        let replacements = (0..3).map(|_| world.spawn((Marker,))).collect::<Vec<_>>();
        for entity in &replacements {
            commands.despawn(*entity);
        }
        match &commands.queue[0] {
            Command::EntityBatch(batch) => {
                assert_eq!(batch.entries.capacity(), entries_capacity);
                assert_eq!(batch.index.capacity(), index_capacity);
            }
            _ => panic!("expected the reused entity batch"),
        }
    }

    #[test]
    fn typed_spawn_batches_reuse_bundle_capacity() {
        let mut world = World::new();
        let mut commands = CommandBuffer::new();
        for _ in 0..64 {
            commands.spawn((Marker,));
        }

        let capacity = match &mut commands.queue[0] {
            Command::SpawnBatch(batch) => batch
                .as_any_mut()
                .downcast_mut::<TypedSpawnBatch<(Marker,)>>()
                .unwrap()
                .bundles
                .capacity(),
            _ => panic!("expected a typed spawn batch"),
        };

        commands.apply(&mut world);
        assert_eq!(world.entity_count(), 64);
        match &mut commands.queue[0] {
            Command::SpawnBatch(batch) => {
                let batch = batch
                    .as_any_mut()
                    .downcast_mut::<TypedSpawnBatch<(Marker,)>>()
                    .unwrap();
                assert!(batch.bundles.is_empty());
                assert_eq!(batch.bundles.capacity(), capacity);
            }
            _ => panic!("expected the reusable spawn batch"),
        }

        for _ in 0..64 {
            commands.spawn((Marker,));
        }
        match &mut commands.queue[0] {
            Command::SpawnBatch(batch) => assert_eq!(
                batch
                    .as_any_mut()
                    .downcast_mut::<TypedSpawnBatch<(Marker,)>>()
                    .unwrap()
                    .bundles
                    .capacity(),
                capacity
            ),
            _ => panic!("expected the reused spawn batch"),
        }
    }

    struct DropCounter(Arc<AtomicUsize>);

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct PanicOnDrop(Arc<AtomicUsize>);

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
            panic!("secondary cleanup panic");
        }
    }

    #[test]
    fn apply_panic_discards_unvisited_reusable_slots() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut world = World::new();
        let mut commands = CommandBuffer::new();
        commands.push_deferred(|_| {});
        commands.push_deferred(|_| panic!("intentional deferred command panic"));
        commands.spawn((DropCounter(drops.clone()),));

        let result = catch_unwind(AssertUnwindSafe(|| commands.apply(&mut world)));

        assert!(result.is_err());
        assert!(commands.is_empty());
        assert_eq!(commands.active_commands, 0);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert!(world.is_poisoned());
    }

    #[test]
    fn apply_panic_swallows_panics_from_unvisited_command_drops() {
        let panicking_drops = Arc::new(AtomicUsize::new(0));
        let ordinary_drops = Arc::new(AtomicUsize::new(0));
        let mut world = World::new();
        let mut commands = CommandBuffer::new();

        commands.push_deferred(|_| panic!("primary command panic"));
        let panic_on_drop = PanicOnDrop(panicking_drops.clone());
        commands.push_deferred(move |_| {
            let _ = &panic_on_drop;
        });
        commands.spawn((DropCounter(ordinary_drops.clone()),));

        let result = catch_unwind(AssertUnwindSafe(|| commands.apply(&mut world)));
        let payload = result.expect_err("the primary command must still unwind");

        assert_eq!(
            payload.downcast_ref::<&'static str>(),
            Some(&"primary command panic")
        );
        assert_eq!(panicking_drops.load(Ordering::Relaxed), 1);
        assert_eq!(ordinary_drops.load(Ordering::Relaxed), 1);
        assert!(commands.is_empty());
        assert_eq!(commands.active_commands, 0);
        assert!(world.is_poisoned());
    }
}

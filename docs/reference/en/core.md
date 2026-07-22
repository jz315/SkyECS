# Core entities and storage

[API index](../../API.md) · [中文](../zh-CN/core.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## Synopsis

```rust
pub struct World {
    pub time: Time,
    // private fields
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId { /* private fields */ }

pub trait Bundle: sealed::BundleSealed + 'static { /* sealed */ }
pub trait ColumnBundle: sealed::ColumnBundleSealed + 'static { /* sealed */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnLengthMismatch { /* private fields */ }
```

`World` owns entities, components, resources, plugins, and the schedule. `EntityId` is a
World-local generational handle; equality compares both its slot and generation.

## `World` members

### Construction and state

| Declaration | Effect |
|---|---|
| `pub fn new() -> Self` | Constructs an empty World with built-in `Time` and the built-in stages. |
| `impl Default for World` | Equivalent to `World::new()`. |
| `pub fn is_poisoned(&self) -> bool` | Reports whether a deferred-command panic poisoned the World. |
| `pub fn entity_count(&self) -> usize` | Returns the live entity count. |
| `pub fn archetype_count(&self) -> usize` | Returns the number of storage archetypes currently present. |
| `pub fn clear(&mut self)` | Drops all entities and components; resources and schedule configuration remain. |

`World::clear` completes storage cleanup before resuming the first component-destructor
panic. A poisoned World remains inspectable and may be shut down, but rejects later command
application and schedule ticks.

### Entity and component operations

| Declaration | Return value / effect |
|---|---|
| `pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityId` | Creates one entity from a component tuple. |
| `pub fn spawn_batch<B: Bundle>(&mut self, bundles: impl IntoIterator<Item = B>)` | Creates all entities produced by the iterator. |
| `pub fn spawn_columns<C: ColumnBundle>(&mut self, columns: &mut C) -> Result<(), ColumnLengthMismatch>` | Moves equal-length component vectors into storage and leaves their allocations reusable. |
| `pub fn contains(&self, entity: EntityId) -> bool` | `true` only for a live matching generation. |
| `pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_` | Iterates live IDs in dense storage order. |
| `pub fn has<T: 'static>(&self, entity: EntityId) -> bool` | Tests whether a live entity contains `T`. |
| `pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T>` | Returns `T`, or `None` for an invalid/stale ID or missing component. |
| `pub fn get_mut<T: 'static>(&mut self, entity: EntityId) -> Option<&mut T>` | Mutable counterpart to `get`. |
| `pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) -> bool` | Overwrites `T` in place or migrates the entity; `false` means the entity is invalid. |
| `pub fn remove<T: 'static>(&mut self, entity: EntityId) -> bool` | Removes `T` and migrates the entity; `false` means invalid entity or missing `T`. |
| `pub fn despawn(&mut self, entity: EntityId) -> bool` | Drops all components and invalidates the ID; `false` means invalid/stale. |

`spawn_columns` is transactional on a length mismatch: neither the World nor any input
column is changed. Successful insertion empties every input vector without discarding its
capacity. `insert` installs a replacement before resuming a panic from the old value's
destructor. `remove` and `despawn` repair storage and entity routes before resuming a
destructor panic.

### Complexity and allocation

| Operation | Complexity |
|---|---|
| `contains`, `has`, `get`, `get_mut` | Constant entity-route lookup plus a component lookup bounded by the 32-component archetype limit; no allocation. |
| `entities` | O(number of live entities + visited chunks/archetypes); the iterator itself does not collect. |
| `spawn` | Amortized constant metadata work plus component moves; may allocate or grow storage. |
| `spawn_batch`, `spawn_columns` | O(number of rows × number of bundle columns); storage allocation is batched. |
| `insert`, `remove` | In-place overwrite is bounded component work; an archetype change copies/moves the surviving component columns and may allocate. |
| `clear` | O(live component values + allocated storage objects). |

Any structural operation can change dense iteration order through swap removal.

## `EntityId`

| Declaration | Meaning |
|---|---|
| `pub const fn new(index: u32, generation: u32) -> Self` | Constructs a raw handle; it is not guaranteed to be live in any World. |
| `pub fn index(self) -> u32` | Returns the reusable slot index. |
| `pub fn generation(self) -> u32` | Returns the generation used for stale-handle detection. |

An `EntityId` is not a persistent document/network identity and is meaningful only when
validated against the originating World.

## `Bundle` and `ColumnBundle`

Sky ECS implements both sealed traits for tuples with one to sixteen distinct `'static`
component types. Users consume these implementations but cannot provide new ones.

`Bundle` exposes the following low-level members; normal code uses `World::spawn` or
`World::spawn_batch`:

```rust
fn cached_meta() -> (Archetype, &'static [(usize, usize)]);
fn archetype() -> Archetype;
unsafe fn write(self, chunk: &mut Chunk, entity_index: usize);
unsafe fn write_fast(
    self,
    chunk: &mut Chunk,
    entity_index: usize,
    columns: &[(usize, usize)],
);
```

Calling either unsafe writer requires a matching archetype and a valid uninitialized row.
`ColumnBundle` has no public members.

`ColumnLengthMismatch` provides:

| Declaration | Value |
|---|---|
| `pub fn column_index(self) -> usize` | Zero-based mismatching column. |
| `pub fn expected(self) -> usize` | Row count established by the first column. |
| `pub fn actual(self) -> usize` | Row count of the mismatching column. |

## Panics

- Entity slot exhaustion panics.
- Duplicate component identities are rejected. Tuple bundles wider than 16 have no trait
  implementation; expert-built archetypes wider than 32 panic during `build`.
- User component destructors may panic; the ownership guarantees described above still apply.

## Minimal example

```rust
use sky_ecs::World;

struct Position(f32);

let mut world = World::new();
let entity = world.spawn((Position(1.0),));
assert_eq!(world.get::<Position>(entity).map(|p| p.0), Some(1.0));
assert!(world.remove::<Position>(entity));
assert!(!world.has::<Position>(entity));
```

## See also

- [Entity access](entity-access.md)
- [Queries](queries.md)
- [Commands](commands.md)
- [Resources](resources.md)
- [Expert storage API](expert.md)

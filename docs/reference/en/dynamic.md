# Dynamic ECS API

[API index](../../API.md) · [中文](../zh-CN/dynamic.md) · [Rustdoc](https://docs.rs/sky_ecs/latest/sky_ecs/dynamic/)

Module: `sky_ecs::dynamic`

The dynamic API is safe and runtime-typed. It validates component identity, slot access mode,
optionality, and aliasing before constructing typed slices.

## Dynamic spawning

```rust
pub struct ErasedComponentValue { /* private fields */ }
pub struct DynamicBundle { /* private fields */ }

pub trait WorldDynamicExt {
    fn spawn_dynamic(
        &mut self,
        bundle: DynamicBundle,
    ) -> Result<EntityId, DynamicSpawnError>;
}

#[derive(Debug)]
pub enum DynamicSpawnError {
    TooManyComponents { count: usize, max: usize },
    DuplicateComponent { component: ComponentType },
}
```

The public limits are `MAX_DYNAMIC_BUNDLE_COMPONENTS` (32) and
`MAX_DYNAMIC_QUERY_SLOTS` (16). Inputs beyond either limit return an error
before archetype or query-plan construction.

| Type | Members |
|---|---|
| `ErasedComponentValue` | `from_typed<T: 'static>(value: T) -> Self`; `component(&self) -> ComponentType` |
| `DynamicBundle` | `new() -> Self`; `with<T: 'static>(self, value: T) -> Self`; `from_values(Vec<ErasedComponentValue>) -> Result<Self, DynamicSpawnError>`; `len`; `is_empty` |

`DynamicBundle::with` records values without immediate duplicate validation.
`from_values` and `WorldDynamicExt::spawn_dynamic` reject duplicate component identities.
Ownership of every successful bundle value transfers to the World; values in a rejected bundle
are dropped normally.

## Query construction

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicAccess { Read, Write }

pub struct DynamicQueryBuilder { /* private fields */ }
pub struct DynamicQuery { /* private fields */ }
```

`DynamicQuery::builder()` and `DynamicQueryBuilder::new()` create an empty builder.

| Builder member | Added slot |
|---|---|
| `read<T: 'static>(self) -> Self` | Required shared `T`. |
| `write<T: 'static>(self) -> Self` | Required exclusive `T`. |
| `optional_read<T: 'static>(self) -> Self` | Optional shared `T`. |
| `optional_write<T: 'static>(self) -> Self` | Optional exclusive `T`. |
| `read_component(self, ComponentType) -> Self` | Runtime-typed required read. |
| `write_component(self, ComponentType) -> Self` | Runtime-typed required write. |
| `optional_read_component(self, ComponentType) -> Self` | Runtime-typed optional read. |
| `optional_write_component(self, ComponentType) -> Self` | Runtime-typed optional write. |
| `build(self) -> Result<DynamicQuery, DynamicQueryError>` | Validates width and unique component identity, then creates a cached query. |

One component identity may occupy only one slot.

## `DynamicQuery`

| Declaration | Contract |
|---|---|
| `slot_count(&self) -> usize` | Number of declared slots. |
| `has_writes(&self) -> bool` | Whether any slot is writable. |
| `for_each_chunk<F>(&mut self, world: &World, f: F) -> Result<(), DynamicQueryError>` | Read-only execution; rejects a query containing write slots. |
| `for_each_chunk_mut<F>(&mut self, world: &mut World, f: F) -> Result<(), DynamicQueryError>` | Executes read/write queries. |

The callback returns `Result<(), DynamicQueryError>`. Iteration stops after the first callback
error. Query match metadata refreshes automatically when the World changes.

## Chunk views

`DynamicQueryChunk<'w>` and `DynamicQueryChunkMut<'w>` both provide `len`, `is_empty`,
`entities`, `component(slot)`, `read<T>(slot)`, and `optional_read<T>(slot)`.

Read-only chunks additionally provide:

```rust
pub fn read_pair<A: 'static, B: 'static>(
    &self,
    a: usize,
    b: usize,
) -> Result<(&'w [A], &'w [B]), DynamicQueryError>;
```

Mutable chunks additionally provide:

```rust
pub fn write<T: 'static>(&mut self, slot: usize) -> Result<&mut [T], DynamicQueryError>;
pub fn optional_write<T: 'static>(
    &mut self,
    slot: usize,
) -> Result<Option<&mut [T]>, DynamicQueryError>;
pub fn write_read<A: 'static, B: 'static>(
    &mut self,
    write_slot: usize,
    read_slot: usize,
) -> Result<(&mut [A], &[B]), DynamicQueryError>;
pub fn write_write<A: 'static, B: 'static>(
    &mut self,
    left_slot: usize,
    right_slot: usize,
) -> Result<(&mut [A], &mut [B]), DynamicQueryError>;
pub fn write_optional_read<A: 'static, B: 'static>(
    &mut self,
    write_slot: usize,
    read_slot: usize,
) -> Result<(&mut [A], Option<&[B]>), DynamicQueryError>;
```

Entity and component slices are row-aligned and have length `len()`. Multi-slice methods require
distinct slot indices; use them whenever multiple live slices are needed from one mutable chunk.

## `DynamicQueryError`

| Variant | Condition |
|---|---|
| `TooManySlots { count, max }` | Builder exceeds `MAX_DYNAMIC_QUERY_SLOTS`. |
| `DuplicateComponent { component }` | Builder contains the same component identity twice. |
| `InvalidSlot { slot, slot_count }` | Slot is outside the query. |
| `ComponentMismatch { slot, expected, actual }` | Requested Rust type does not match slot metadata. |
| `RequiresMutableWorld` | Read-only executor used for a query with writes. |
| `RequiresWriteAccess { slot }` | A write getter targets a read-only slot. |
| `MissingRequiredComponent { slot }` | A required getter targets an absent optional column. |
| `SlotAlias { left, right }` | A multi-slice method received the same slot twice. |

## Complexity and allocation

- Building validates uniqueness in O(number of slots²); query widths are intended to remain
  small and are bounded by `MAX_DYNAMIC_QUERY_SLOTS`.
- Initial/invalidated execution prepares matching archetypes; chunk callbacks then perform
  constant-time validated slot lookup plus slice construction.
- Iteration allocates no per-entity objects and invokes the callback once per matching chunk.

## Minimal example

```rust
use sky_ecs::dynamic::DynamicQuery;
use sky_ecs::World;

struct Position(f32);
struct Velocity(f32);

let mut world = World::new();
world.spawn((Position(1.0), Velocity(2.0)));

let mut query = DynamicQuery::builder()
    .write::<Position>()
    .read::<Velocity>()
    .build()
    .unwrap();

query.for_each_chunk_mut(&mut world, |mut chunk| {
    let (positions, velocities) = chunk.write_read::<Position, Velocity>(0, 1)?;
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
    Ok(())
}).unwrap();
```

## See also

- [Runtime component types](plugins-types.md)
- [Typed queries](queries.md)
- [Unsafe expert API](expert.md)

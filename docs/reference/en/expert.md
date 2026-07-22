# Expert storage API

[API index](../../API.md) · [中文](../zh-CN/expert.md) · [Rustdoc](https://docs.rs/sky_ecs/latest/sky_ecs/expert/)

Module: `sky_ecs::expert`

This module exposes storage invariants directly. Gameplay and ordinary tools should prefer the
typed API or [`dynamic`](dynamic.md).

## Exports

```rust
pub use /* ... */ {
    Archetype,
    ArchetypeBuilder,
    Chunk,
    ComponentType,
    ComponentTypeInfo,
    PreparedQuery,
};

pub fn create_archetype() -> ArchetypeBuilder;
pub fn component_type<T: 'static>() -> ComponentType;
pub fn register_component_type(name: &str, size: usize, align: usize) -> ComponentType;
pub fn interned_archetype_count() -> usize;

pub trait WorldExpertExt {
    unsafe fn spawn_uninit(&mut self, archetype: Archetype) -> EntityId;
}
```

`ComponentType` and `PreparedQuery` have the same contracts as their crate-root exports.

## `ArchetypeBuilder` and `Archetype`

| Declaration | Effect |
|---|---|
| `create_archetype() -> ArchetypeBuilder` | Starts an empty component signature. |
| `ArchetypeBuilder::add_component(self, ty: ComponentType) -> Self` | Adds runtime type metadata. |
| `ArchetypeBuilder::add_rust_component<T: 'static>(self) -> Self` | Adds `component_type::<T>()`. |
| `ArchetypeBuilder::build(self) -> Archetype` | Sorts, validates, and process-interns the signature. |
| `Archetype::id(&self) -> usize` | Process-local identity derived from interned metadata. |
| `interned_archetype_count() -> usize` | Number of signatures retained by the process. |

`Archetype` is `Copy + Eq + Hash` and dereferences to immutable metadata exposing
`components`, `alignment`, `has_component`, and `query_component_index`. Interned metadata
is intentionally never reclaimed. Building rejects duplicate component identities and more
than 32 components.

## `Chunk`

```rust
pub struct Chunk {
    pub entity_count: usize,
    pub max_entity_count: usize,
    pub archetype: Archetype,
    // private allocation metadata
}
```

| Declaration | Contract |
|---|---|
| `Chunk::new(archetype: Archetype) -> Self` | Allocates a chunk using the default layout policy. |
| `is_full(&self) -> bool` | `entity_count == max_entity_count`. |
| `is_empty(&self) -> bool` | `entity_count == 0`. |
| `unsafe add_entity(&mut self, entity: EntityId) -> Option<usize>` | Reserves a logical row; caller must initialize every component before observation or drop. |
| `column_ptr(&self, component_index: usize) -> *mut u8` | Raw column base; index must be valid. |
| `data_ptr(&self) -> *mut u8` | Raw allocation base (or aligned dangling address for all-ZST layouts). |
| `component_ptr(&self, component_index: usize, entity_index: usize) -> *mut u8` | Row pointer; returns null for an out-of-range entity row, but component index must be valid. |
| `get_entity_as_ptr(&self, index: usize) -> *const u8` | First-component row pointer, or null for an empty archetype/out-of-range row. |
| `entity_id(&self, entity_index: usize) -> Option<EntityId>` | ID at a logical row. |

Raw pointer functions do not create Rust references and do not validate type identity,
initialization, aliasing, or most component-index preconditions.

## `WorldExpertExt::spawn_uninit`

```rust
unsafe fn spawn_uninit(&mut self, archetype: Archetype) -> EntityId;
```

The returned entity is already registered in the World. Before it is queried, moved, removed,
or dropped, the caller must initialize every component slot described by `archetype` exactly
once. Violating this contract can cause invalid reads or destructor calls on uninitialized data.

## Lifetime, allocation, and complexity

- Archetype handles and component type handles reference process-lifetime interned metadata.
- Creating a new signature/type may allocate and intentionally retain metadata.
- Reusing an existing archetype is a registry lookup; handle equality/hash is constant time.
- Chunk construction allocates according to its layout except all-ZST layouts, which use an
  aligned dangling component address and logical entity storage.
- Pointer accessors are constant-time and allocation-free.

## See also

- [Core safe API](core.md)
- [Dynamic safe API](dynamic.md)
- [Component type registry](plugins-types.md)

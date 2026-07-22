# Plugins, component types, and derive macros

[API index](../../API.md) · [中文](../zh-CN/plugins-types.md) · [Rustdoc](https://docs.rs/sky_ecs)

Modules: `sky_ecs`, `sky_ecs::plugin`

## Plugin protocol

```rust
pub type PluginResult = Result<(), PluginError>;

pub struct PluginError {
    pub plugin: &'static str,
    pub message: String,
}

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn install(self, world: &mut World) -> PluginResult
    where
        Self: Sized;
}

#[derive(Default)]
pub struct PluginRegistry { /* private fields */ }
```

`PluginError::new(plugin, message)` constructs an error and it implements
`Display + Error`.

`PluginRegistry` members:

| Declaration | Effect |
|---|---|
| `contains<P: 'static>(&self) -> bool` | Whether plugin type `P` was recorded. |
| `get<P: 'static>(&self) -> Option<&'static str>` | Installed name for `P`. |
| `insert<P: 'static>(&mut self, name: &'static str) -> Result<(), &'static str>` | Records `P` or returns the existing name. |

World entry points:

| Declaration | Effect |
|---|---|
| `install<P: Plugin + 'static>(&mut self, plugin: P) -> PluginResult` | Runs installation, then records the concrete plugin type; duplicate installation is an error. |
| `has_plugin<P: 'static>(&self) -> bool` | Tests World-local installation. |
| `require_plugin<P: 'static>(&self, plugin: &'static str) -> PluginResult` | Dependency check producing a `PluginError` named for the caller. |

Plugin installation is not transactional: a failing `Plugin::install` may already have changed
the World. The registry records only installation identity, not configuration or capabilities.

## Component type metadata

```rust
pub type ComponentType = sky_type::Type;
pub type ComponentTypeInfo = sky_type::TypeInfo;

pub fn component_type<T: 'static>() -> ComponentType;
pub fn register_component_type(name: &str, size: usize, align: usize) -> ComponentType;
pub fn component_type_by_name(name: &str) -> Option<ComponentType>;
pub fn component_type_by_rust_type<T: 'static>() -> Option<ComponentType>;
pub fn registered_component_types() -> Vec<ComponentType>;
```

`ComponentType` is a copyable process-interned handle. It dereferences to:

```rust
pub struct ComponentTypeInfo {
    pub size: usize,
    pub align: usize,
    pub name: String,
    pub drop_fn: Option<unsafe fn(*mut u8)>,
    // Rust TypeId is private
}
```

| Member | Meaning |
|---|---|
| `id(&self) -> usize` | Process-local metadata identity used for equality and hashing. |
| `needs_drop(&self) -> bool` | Whether an erased destructor exists. |
| `drop_fn(&self) -> Option<unsafe fn(*mut u8)>` | Erased destructor; caller must provide a valid initialized value of the registered type. |
| `rust_type_id(&self) -> Option<TypeId>` | Rust identity for typed registrations; `None` for opaque dynamic types. |

`component_type::<T>()` registers or returns exact Rust layout/drop metadata.
`component_type_by_rust_type::<T>()` only queries and does not register.
`register_component_type` creates opaque runtime metadata without a destructor.

Registration panics for an empty name, invalid alignment/layout, incompatible reuse of a name,
or collision between opaque and Rust registrations. Metadata is retained for the process
lifetime. `registered_component_types` allocates an unordered snapshot vector.

## `#[derive(QueryData)]`

```rust
#[derive(sky_ecs::QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
    health: Option<&'w Health>,
}
```

Requirements:

- a struct with named, non-empty fields;
- exactly one lifetime parameter, with no bounds;
- no type/const parameters or `where` clause;
- one to sixteen fields of `&T`, `&mut T`, `Option<&T>`, or
  `Option<&mut T>` using that lifetime;
- no duplicate component type.

The derived type can be used as `Q`. Chunk-level methods still expose the underlying tuple of
component slices; entity-level methods yield the named struct.

## `#[derive(StageLabel)]`

```rust
#[derive(sky_ecs::StageLabel)]
struct Physics;
```

The input must be a non-generic unit struct with no `where` clause. The derive implements
`sky_ecs::StageLabel`; the default stage name is the Rust type name.

## Complexity and thread safety

- Component handle equality/hash and metadata access are O(1).
- First registration may take the global registry write lock and allocate; repeated typed
  lookup uses thread-local caches before the shared registry.
- Plugin registry operations use a World-local type-keyed hash map and are expected O(1).

## See also

- [Dynamic API](dynamic.md)
- [Expert API](expert.md)
- [Scheduling](scheduling.md)
- [Queries](queries.md)

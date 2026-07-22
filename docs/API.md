# Sky ECS API Reference

[中文](API_zh.md) · [Tutorial](TUTORIAL.md) · [Rustdoc](https://docs.rs/sky_ecs) · [Examples](../crates/sky_ecs/examples/README.md)

This is the index for Sky ECS's supported public surface. Each page records declarations,
constraints, return values, errors, borrowing and invalidation rules, and complexity. For a
task-oriented introduction, use the [Tutorial](TUTORIAL.md).

## Reference pages

| Subsystem | Public surface |
|---|---|
| [Core entities and storage](reference/en/core.md) | `World`, `EntityId`, `Bundle`, `ColumnBundle`, `ColumnLengthMismatch`; spawn, direct component access, migration, and destruction |
| [Entity access](reference/en/entity-access.md) | `EntityAccessor{Mut}`, `PreparedEntityAccess{Mut}`, `PreparedEntityView`, bound views, and their constructors |
| [Typed queries](reference/en/queries.md) | `Query`, `QueryMut`, `PreparedQuery`, `QueryFilter`, `With`, `Without`, `Any` |
| [Deferred commands](reference/en/commands.md) | `CommandBuffer` and scheduler-issued `Commands` |
| [Resources](reference/en/resources.md) | World resource methods, `Res`, `ResMut`, and the permanent `Time` resource rules |
| [Systems](reference/en/systems.md) | `IntoSystem`, `ExclusiveSystem`, `View`, `ParView`, `Local`, and system parameter contracts |
| [Scheduling and time](reference/en/scheduling.md) | `StageBuilder`, `StageLabel`, built-in stages, `FixedStep`, `Time`, reports, diagnostics, and schedule errors |
| [Dynamic API](reference/en/dynamic.md) | `sky_ecs::dynamic` safe runtime-typed spawning and chunk queries |
| [Expert API](reference/en/expert.md) | `sky_ecs::expert` archetypes, chunks, uninitialized spawning, and raw storage contracts |
| [Plugins, types, and macros](reference/en/plugins-types.md) | `Plugin` protocol, component type registry, `QueryData`, and `StageLabel` derives |

## Operation index

| Operation | Entry point |
|---|---|
| Occasional component lookup by ID | `World::get` / `World::get_mut` |
| Repeated arbitrary-ID lookup | `World::accessor` / `World::accessor_mut` |
| Repeated access to one fixed ID sequence | `World::prepare_access` / `World::prepare_access_mut` |
| Dense typed traversal | `World::query` / `World::query_mut` |
| Explicit reusable typed query plan | `PreparedQuery` |
| Runtime-selected components | `dynamic::DynamicQuery` |
| Structural changes during systems/iteration | `CommandBuffer` / `Commands` |
| Scheduled component access | `View` / `ParView` |

## Surface boundary

The reference covers crate-root exports, `sky_ecs::dynamic`, `sky_ecs::expert`,
`sky_ecs::stage`, and `sky_ecs::plugin`. It intentionally excludes `__private`, sealed
implementation traits, `pub(crate)` items, and storage internals not re-exported through the
expert module. Public-but-hidden derive support is an implementation contract for generated
code, not a hand-written API.

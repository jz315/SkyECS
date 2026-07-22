# Deferred commands

[API index](../../API.md) · [中文](../zh-CN/commands.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## Synopsis

```rust
#[derive(Default)]
pub struct CommandBuffer { /* private fields */ }

pub struct Commands<'w> { /* private fields */ }
```

`CommandBuffer` is an owned explicit buffer. `Commands` is a scheduler-issued writer backed by
one invocation-private `CommandBuffer`.

## `CommandBuffer` members

| Declaration | Effect |
|---|---|
| `pub fn new() -> Self` | Creates an empty reusable buffer. |
| `pub fn is_empty(&self) -> bool` | Tests the logical queued-command count. |
| `pub fn len(&self) -> usize` | Returns the number of commands recorded, not internal batches. |
| `pub fn spawn<B: Bundle>(&mut self, bundle: B)` | Defers a spawn; consecutive identical bundle types are batched. |
| `pub fn despawn(&mut self, entity: EntityId)` | Defers despawn. |
| `pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T)` | Defers insert or overwrite. |
| `pub fn remove<T: 'static>(&mut self, entity: EntityId)` | Defers component removal. |
| `pub fn insert_resource<R: 'static>(&mut self, resource: R)` | Defers resource insertion. |
| `pub fn remove_resource<R: 'static>(&mut self)` | Defers resource removal. |
| `pub fn apply(&mut self, world: &mut World)` | Applies the active prefix and returns the buffer to empty state. |
| `pub fn clear(&mut self)` | Drops pending payloads while retaining reusable capacity. |

Commands for one entity are coalesced to its final per-component state before structural
migration. Registration/application order remains deterministic.

## `Commands` members

`Commands<'w>` provides the same `is_empty`, `len`, `spawn`, `despawn`, `insert`, `remove`,
`insert_resource`, and `remove_resource` operations. Scheduler safety adds these bounds:

- `spawn` requires `B: Bundle + Send`.
- `insert` requires `T: Send + 'static`.
- `insert_resource` requires `R: Send + 'static`.

It exposes no `apply` or `clear`. The scheduler owns flush boundaries and applies system buffers
in registration order.

## Panic and poisoning contract

`apply` can run user destructors and other user-owned code. If a panic escapes:

- unvisited commands are discarded and each owned payload is dropped at most once;
- the buffer is restored to its empty observable state;
- the World is marked poisoned because general rollback of an arbitrary partial mutation is not
  possible;
- the poisoned World rejects later command application and schedule ticks, but remains
  inspectable and can be shut down.

Applying to an already poisoned World panics. `World::is_poisoned` reports this state.

## Complexity and allocation

- Recording is amortized O(1); the buffer reuses internal slots after `apply` or `clear`.
- Consecutive same-type spawns and consecutive entity commands are stored in batches.
- `apply` is O(recorded commands + affected component data); coalescing may reduce the number of
  archetype migrations.
- `clear` is O(recorded owned payloads) and retains buffer capacity.

## Minimal example

```rust
use sky_ecs::{CommandBuffer, World};

struct Health(u32);

let mut world = World::new();
let entity = world.spawn((Health(100),));
let mut commands = CommandBuffer::new();
commands.insert(entity, Health(50));
commands.apply(&mut world);

assert_eq!(world.get::<Health>(entity).map(|h| h.0), Some(50));
```

## See also

- [Core entity operations](core.md)
- [Systems and system parameters](systems.md)
- [Scheduling and flush boundaries](scheduling.md)

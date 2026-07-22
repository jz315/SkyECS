# Systems and system parameters

[API index](../../API.md) · [中文](../zh-CN/systems.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## Synopsis

```rust
pub trait IntoSystem<Marker>: /* sealed */ + Send + 'static {}

pub trait ExclusiveSystem: 'static {
    fn init(&mut self, _world: &mut World) {}
    fn run(&mut self, world: &mut World);
    fn teardown(&mut self, _world: &mut World) {}
}

pub struct View<'w, Q, F = ()> { /* private fields */ }
pub struct ParView<'w, Q, F = ()> { /* private fields */ }
pub struct Res<'w, T: 'static>(/* private */);
pub struct ResMut<'w, T: 'static>(/* private */);
pub struct Local<'w, T: 'static>(/* private */);
```

## Function systems

`IntoSystem` is implemented automatically for `Send + 'static` functions and closures with
zero to sixteen supported parameters. It is sealed and has no user-callable members.

Supported parameters:

| Parameter | Constraint and access |
|---|---|
| `View<Q, F>` | Sequential typed component access declared by `Q` and `F`. |
| `ParView<Q, F>` | The same component access plus serially prepared parallel jobs. |
| `Res<T>` | Shared resource access; `T: Sync + 'static`. |
| `ResMut<T>` | Exclusive resource access; `T: Send + 'static`; unavailable for `Time`. |
| `Local<T>` | Per-system persistent state initialized with `T::default()`; `T: Default + Send + 'static`. |
| `Commands` | Invocation-private deferred structural writer. |
| `()` | No access. |

The scheduler derives component/resource conflicts from parameter types. A system cannot declare
overlapping mutable capabilities in one parameter tuple.

## `View`

`View<'w, Q, F>` provides:

| Declaration | Callback |
|---|---|
| `for_each<Func>(&self, f: Func)` | `Func: for<'a> FnMut(Q::Item<'a>)` |
| `for_each_with_entity<Func>(&self, f: Func)` | Adds `EntityId`. |
| `for_each_chunk<Func>(&self, f: Func)` | Receives `Q::Chunk<'a>`. |
| `for_each_chunk_with_entities<Func>(&self, f: Func)` | Receives aligned ID and component slices. |
| `count(&self) -> usize` | Matching live row count. |
| `is_empty(&self) -> bool` | Whether no live row matches. |
| `cached_archetype_count(&self) -> usize` | Prepared matching archetype count. |

`Q: QuerySpec` and `F: QueryFilter`. Query/cache preparation happens in the scheduler's serial
prepare phase. Recursive iteration of the same `View` panics.

## `ParView`

`ParView<'w, Q, F>` provides:

| Declaration | Callback |
|---|---|
| `par_for_each<Func>(&self, f: Func)` | Parallel entity items. |
| `par_for_each_with_entity<Func>(&self, f: Func)` | Parallel entity IDs and items. |
| `par_for_each_chunk<Func>(&self, f: Func)` | Parallel typed chunk slices. |
| `par_for_each_chunk_with_entities<Func>(&self, f: Func)` | Parallel aligned IDs and slices. |
| `count(&self) -> usize` | Matching live row count. |
| `is_empty(&self) -> bool` | Whether no live row matches. |
| `cached_archetype_count(&self) -> usize` | Prepared matching archetype count. |

Parallel item/chunk values must be `Send` and callbacks must be `Send + Sync`. The executor may
fall back to sequential processing for small workloads. Execution order is unspecified.
Recursive iteration of the same `ParView` panics.

## `Res`, `ResMut`, and `Local`

- `Res<T>` implements `Deref<Target = T>`.
- `ResMut<T>` implements `Deref<Target = T>` and `DerefMut`.
- `Local<T>` implements `Deref<Target = T>` and `DerefMut`.
- A `Local<T>` value belongs to one registered system, survives between invocations, and is
  dropped when that system is torn down.

Resource rules are specified in [Resources](resources.md).

## `ExclusiveSystem`

Exclusive systems are serial barriers and receive `&mut World`:

| Member | Invocation |
|---|---|
| `init` | Once before the first run after registration/shutdown. |
| `run` | At the system's position in its stage. |
| `teardown` | During `World::shutdown`, in reverse schedule order. |

Any `FnMut(&mut World) + 'static` implements `ExclusiveSystem` with only `run`. Exclusive
systems do not need to be `Send` because they are never dispatched as an ordinary parallel wave.

## Borrowing, errors, and complexity

- Parameter references cannot escape one system invocation.
- Ordinary systems cannot structurally mutate the World directly; use `Commands`.
- Missing `Res`/`ResMut` parameters produce `ScheduleError::MissingResource` during frame
  preflight.
- Prepared query/resource state is retained per registered system and refreshed after the
  relevant World epoch changes.
- Per-invocation traversal complexity matches the corresponding query operation; scheduler
  preparation is outside the component inner loop.

## Minimal example

```rust
use sky_ecs::{Commands, Local, Res, Update, View, World};

struct Position(f32);
struct Step(f32);
struct Marker;

fn advance(
    positions: View<&mut Position>,
    step: Res<Step>,
    mut runs: Local<u32>,
    mut commands: Commands,
) {
    positions.for_each(|position| position.0 += step.0);
    *runs += 1;
    if *runs == 1 {
        commands.spawn((Marker,));
    }
}

let mut world = World::new();
world.insert_resource(Step(1.0));
world.spawn((Position(0.0),));
world.stage(Update).add(advance);
world.tick_with_delta(1.0 / 60.0).unwrap();
```

## See also

- [Queries](queries.md)
- [Commands](commands.md)
- [Resources](resources.md)
- [Scheduling](scheduling.md)

# Resources

[API index](../../API.md) · [中文](../zh-CN/resources.md) · [Rustdoc](https://docs.rs/sky_ecs)

Module: `sky_ecs`

## World resource members

```rust
pub fn World::insert_resource<R: 'static>(&mut self, resource: R) -> Option<R>;
pub fn World::get_resource<R: 'static>(&self) -> Option<&R>;
pub fn World::get_resource_mut<R: 'static>(&mut self) -> Option<&mut R>;
pub fn World::contains_resource<R: 'static>(&self) -> bool;
pub fn World::remove_resource<R: 'static>(&mut self) -> Option<R>;
```

Resources are keyed by their Rust `TypeId`. At most one value of each Rust type is stored.
`insert_resource` returns the replaced value; `remove_resource` returns the removed value.
The lookup methods return `None` when the type is absent.

`Time` is permanent built-in World state:

- `contains_resource::<Time>()` is always `true`;
- `get_resource::<Time>()` and `get_resource_mut::<Time>()` address `World::time`;
- `insert_resource::<Time>` and `remove_resource::<Time>` panic.

Exclusive code may mutate `Time` between ticks. Ordinary scheduled systems can request
`Res<Time>` but not `ResMut<Time>`.

## System resource parameters

```rust
pub struct Res<'w, T: 'static>(/* private */);
pub struct ResMut<'w, T: 'static>(/* private */);
```

| Parameter | Constraint | Access |
|---|---|---|
| `Res<'w, T>` | `T: Sync + 'static` | Shared; implements `Deref<Target = T>`. |
| `ResMut<'w, T>` | `T: Send + 'static` | Exclusive; implements `Deref` and `DerefMut`. |

The scheduler validates resource availability before running a frame. A missing parameter
produces `ScheduleError::MissingResource { system, resource }` before time advances or any
system runs. Resource read/write conflicts participate in wave construction.

## Deferred resource operations

`CommandBuffer` and `Commands` provide `insert_resource` and `remove_resource`. Deferred
resource changes obey the command flush and World-poisoning rules described in
[Deferred commands](commands.md).

## Complexity and invalidation

- Direct resource lookup, insertion, and removal use a type-keyed hash map: expected O(1).
- Insert/remove may allocate or deallocate; lookup does not allocate.
- Insertion/removal increments the resource epoch. Prepared system resource pointers are reused
  only for the same World and unchanged resource epoch.
- Replacing a resource invalidates previously prepared scheduler metadata before the next
  invocation; normal Rust borrowing prevents direct references from surviving the mutation.

## Minimal example

```rust
use sky_ecs::{Res, Update, World};

struct Gravity(f32);

fn read_gravity(gravity: Res<Gravity>) {
    assert_eq!(gravity.0, 9.8);
}

let mut world = World::new();
world.insert_resource(Gravity(9.8));
world.stage(Update).add(read_gravity);
world.tick_with_delta(1.0 / 60.0).unwrap();
```

## See also

- [Systems](systems.md)
- [Scheduling](scheduling.md)
- [`Time`](scheduling.md#time)

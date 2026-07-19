# Sky ECS API Guide

[中文](API_zh.md) · [Tutorial](TUTORIAL.md) · [Rustdoc](https://docs.rs/sky_ecs)

This guide explains the API in the order it normally appears in code. Every
section defines the types and `World` it needs; no block assumes that variables
were defined in an earlier example.

Common types are exported directly from `sky_ecs`; plugin, runtime-typed, and
low-level APIs are also grouped under `sky_ecs::plugin`, `sky_ecs::dynamic`, and
`sky_ecs::expert`.

## 1. Run a complete program first

`Cargo.toml`:

```toml
[dependencies]
sky_ecs = "0.1.3"
```

`src/main.rs`:

```rust
use sky_ecs::World;

#[derive(Debug)]
struct Position { x: f32, y: f32 }

#[derive(Debug)]
struct Velocity { x: f32, y: f32 }

fn main() {
    let mut world = World::new();

    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 2.0 },
    ));

    world
        .query_mut::<(&mut Position, &Velocity)>()
        .for_each(|(position, velocity)| {
            position.x += velocity.x;
            position.y += velocity.y;
        });

    world.query::<&Position>().for_each(|position| {
        println!("{position:?}");
    });
}
```

This program already contains the three core ECS steps:

1. `World::new` creates the container.
2. `spawn` creates an entity from a component tuple.
3. `query_mut` modifies components, while `query` iterates them read-only.

## 2. World, components, Bundle, and EntityId

`World` owns entities, components, resources, and the scheduler. Any `'static`
Rust type can be a component. The component tuple passed to `spawn` is a Bundle.

```rust
use sky_ecs::{EntityId, World};

struct Position(f32, f32);
struct Health(u32);
struct Player; // Zero-sized marker component

let mut world = World::new();
let player: EntityId = world.spawn((Position(0.0, 0.0), Health(100), Player));

assert!(world.contains(player));
assert_eq!(world.entity_count(), 1);
```

`EntityId` contains an index and a generation. After an entity is despawned,
its old ID becomes invalid. Reusing the underlying slot never makes an old ID
refer to a new entity.

### Batch creation

Use `spawn_batch` when many entities have the same Bundle shape:

```rust
use sky_ecs::World;

struct Position(f32, f32);
struct Velocity(f32, f32);

let mut world = World::new();
world.spawn_batch((0..10_000).map(|i| (
    Position(i as f32, 0.0),
    Velocity(1.0, 0.0),
)));

assert_eq!(world.entity_count(), 10_000);
```

`spawn_batch` is intended for bulk imports that do not need to retain every
`EntityId` immediately.

If the producer already stores one `Vec` per component, use `spawn_columns`
to import that structure-of-arrays data directly:

```rust
use sky_ecs::World;

struct Position(f32, f32);
struct Velocity(f32, f32);

let mut columns = (
    vec![Position(0.0, 0.0), Position(1.0, 0.0)],
    vec![Velocity(1.0, 0.0), Velocity(2.0, 0.0)],
);
let position_capacity = columns.0.capacity();

let mut world = World::new();
world.spawn_columns(&mut columns).unwrap();

assert_eq!(world.entity_count(), 2);
assert!(columns.0.is_empty());
assert_eq!(columns.0.capacity(), position_capacity);
```

All columns must have the same length. On success their values move into the
World, while the now-empty source `Vec`s retain their allocations for reuse.
A length mismatch returns `ColumnLengthMismatch` before changing either the
World or the source columns. Use `spawn_batch` when data is naturally produced
as entity Bundles; use `spawn_columns` for loaders, deserializers, and other
code that naturally produces separate component arrays.

Component tuples used by `spawn`, `spawn_batch`, `spawn_columns`, typed
queries, and filters support up to 16 items. One Archetype can contain at most
32 distinct component types. This is a per-Archetype storage limit, not a limit
on how many component types a `World` may register. The wider low-level limit
mainly serves dynamic and expert construction paths.

## 3. Read and write one entity

When you know the `EntityId`, use these methods:

| Operation | Method | On failure |
|---|---|---|
| Check an entity | `contains(entity)` | `false` |
| Check a component | `has::<T>(entity)` | `false` |
| Read a component | `get::<T>(entity)` | `None` |
| Modify a component | `get_mut::<T>(entity)` | `None` |
| Add or replace | `insert(entity, value)` | `false` |
| Remove a component | `remove::<T>(entity)` | `false` |
| Despawn an entity | `despawn(entity)` | `false` |

```rust
use sky_ecs::World;

#[derive(Debug, PartialEq)]
struct Health(u32);
struct Poison;

let mut world = World::new();
let entity = world.spawn((Health(100),));

assert_eq!(world.get::<Health>(entity), Some(&Health(100)));
world.get_mut::<Health>(entity).unwrap().0 -= 10;
assert!(world.insert(entity, Poison));
assert!(world.has::<Poison>(entity));
assert!(world.remove::<Poison>(entity));
assert!(world.despawn(entity));
assert!(world.get::<Health>(entity).is_none());
```

`insert`, `remove`, and `despawn` change storage structure, so they require
`&mut World`.

## 4. Move from single-entity access to queries

Use `get` when an ID is known and accessed only once or twice. Use a query when
you need to process every entity that contains a component combination.

```rust
use sky_ecs::World;

struct Position { x: f32, y: f32 }
struct Velocity { x: f32, y: f32 }

let mut world = World::new();
world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
world.spawn((Position { x: 5.0, y: 0.0 },)); // No Velocity

// Only entities with both Position and Velocity match.
world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });

// Use a with_entity variant when the query item also needs EntityId.
world.query::<&Position>().for_each_with_entity(|entity, position| {
    println!("{entity:?}: ({}, {})", position.x, position.y);
});
```

Use `query` for read-only queries. Use `query_mut` whenever the query item
contains `&mut T`.

## 5. Optional components and filters

`Option<&T>` means “match the entity even when T is absent.” `With<T>` and
`Without<T>` mean “select Archetypes that contain or do not contain T.”

```rust
use sky_ecs::{With, Without, World};

struct Position(f32, f32);
struct Velocity(f32, f32);
struct Enemy;
struct Disabled;

let mut world = World::new();
world.spawn((Position(0.0, 0.0), Velocity(1.0, 0.0), Enemy));
world.spawn((Position(5.0, 0.0), Enemy));
world.spawn((Position(9.0, 0.0), Enemy, Disabled));

// All three entities match; velocity may be None.
world
    .query::<(&Position, Option<&Velocity>)>()
    .filter::<With<Enemy>>()
    .for_each(|(position, velocity)| {
        let _ = (position, velocity);
    });

// Only enemies that are not disabled match.
let active_enemies = world
    .query::<&Position>()
    .filter::<(With<Enemy>, Without<Disabled>)>();
assert_eq!(active_enemies.count(), 2);
```

Use `Any<(With<A>, With<B>)>` for OR filters. Wide query tuples can be replaced
with named fields by using `#[derive(QueryData)]`.

## 6. Entity, chunk, and parallel iteration

| Goal | Method |
|---|---|
| Process each entity | `for_each` |
| Also receive the ID | `for_each_with_entity` |
| Receive aligned component slices | `for_each_chunk` |
| Receive chunk slices and an ID slice | `for_each_chunk_with_entities` |
| Process entities in parallel | `par_for_each` |
| Process chunks in parallel | `par_for_each_chunk` |

```rust
use sky_ecs::World;

struct Position(f32);
struct Velocity(f32);

let mut world = World::new();
world.spawn_batch((0..10_000).map(|_| (Position(0.0), Velocity(1.0))));

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each_chunk(|(positions, velocities)| {
        for i in 0..positions.len() {
            positions[i].0 += velocities[i].0;
        }
    });

world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|(position, velocity)| {
        position.0 += velocity.0;
    });
```

Start ordinary logic with `for_each`. Use chunk iteration when an inner
algorithm needs slices or is easier to vectorize. Parallel iteration has
scheduling overhead, and small workloads automatically fall back to the
sequential path.

## 7. Resources: World-wide singletons

A Resource does not belong to an entity. Resources are suitable for global
configuration, time, scores, and shared state.

```rust
use sky_ecs::World;

#[derive(Default)]
struct Score(u32);

let mut world = World::new();
assert!(world.insert_resource(Score::default()).is_none());
world.get_resource_mut::<Score>().unwrap().0 += 10;
assert_eq!(world.get_resource::<Score>().unwrap().0, 10);
assert!(world.contains_resource::<Score>());
let score = world.remove_resource::<Score>().unwrap();
assert_eq!(score.0, 10);
```

When `insert_resource` replaces an existing value, it returns `Some(old_value)`.

## 8. Commands: deferred structural changes

A live query borrows the World, so it cannot call `spawn`, `despawn`, `insert`,
or `remove` at the same time. Outside the scheduler, use `CommandBuffer` to
record changes and apply them together later:

```rust
use sky_ecs::{CommandBuffer, World};

struct Position(f32, f32);
struct Poison;

let mut world = World::new();
let entity = world.spawn((Position(0.0, 0.0),));

let mut commands = CommandBuffer::new();
commands.insert(entity, Poison);
commands.spawn((Position(5.0, 0.0),));
commands.apply(&mut world);

assert!(world.has::<Poison>(entity));
assert_eq!(world.entity_count(), 2);
```

Inside a system, use the borrowed `Commands<'_>` parameter. The scheduler
flushes it at safe boundaries.

## 9. Systems, resources, and stages

This is a complete runnable scheduling example:

```rust
use sky_ecs::{Res, ResMut, Time, Update, View, World};

struct Position(f32);
struct Velocity(f32);

#[derive(Default)]
struct FrameCount(u32);

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.0 += velocity.0 * time.delta;
    });
}

fn count_frame(mut frames: ResMut<FrameCount>) {
    frames.0 += 1;
}

fn main() {
    let mut world = World::new();
    world.insert_resource(FrameCount::default());
    world.spawn((Position(0.0), Velocity(2.0)));

    world.stage(Update).add(movement).add(count_frame);

    for _ in 0..60 {
        world.tick_with_delta(1.0 / 60.0).unwrap();
    }

    world.shutdown();
    assert_eq!(world.get_resource::<FrameCount>().unwrap().0, 60);
}
```

System parameters are access declarations:

- `View<Q>`: sequential component query.
- `ParView<Q>`: parallel query that can use `par_*` methods.
- `Res<T>` / `ResMut<T>`: read-only / mutable resource.
- `Local<T>`: system-private state retained across frames.
- `Commands`: deferred structural changes.
- `Res<Time>`: the World's permanent built-in read-only time resource.

### `Time`: the built-in time resource

Every `World` contains `Time` from construction; there is no need to call
`insert_resource`. It cannot be replaced or removed, and
`contains_resource::<Time>()` always returns `true`. Ordinary systems read it
through `Res<Time>` and cannot request `ResMut<Time>`, because the scheduler
owns the frame-time updates.

```rust
fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.0 += velocity.0 * time.delta;
    });
}
```

The time values have distinct meanings:

| Field | Meaning |
|---|---|
| `delta` | Step for the current stage invocation. It equals `frame_delta` in every-frame stages and temporarily becomes the fixed step in `FixedUpdate`. |
| `frame_delta` | Current application-frame step after clamping and `time_scale`. Rendering and UI normally use this value. |
| `raw_delta` | Real frame interval before clamping and time scaling. |
| `elapsed` | Accumulated time affected by `time_scale`. |
| `raw_elapsed` | Real accumulated time unaffected by `time_scale`. |
| `fixed_alpha` | Remaining fraction of the built-in fixed-step accumulator, useful for render interpolation. |

`tick()` updates `Time` from the wall clock. `tick_with_delta(delta)` treats
its argument as both raw and frame time, which is useful for tests and
deterministic simulations. An application runner that clamps large frame
intervals can call `tick_with_frame_delta(frame_delta, raw_delta)` to preserve
the real `raw_delta`.

Configure time scaling outside schedule execution:

```rust
let mut world = World::new();
world.get_resource_mut::<Time>().unwrap().time_scale = 0.5;
world.tick_with_delta(1.0 / 60.0).unwrap();
```

This mutates the same built-in `Time` owned by the World. It cannot be replaced
with `insert_resource(Time)` or deleted with `remove_resource::<Time>()`.

Built-in stages run in this order:
`First -> FixedUpdate -> PreUpdate -> Update -> PostUpdate -> Last`.
Put fixed-rate logic in `FixedUpdate` and configure it with
`FixedStep::hz(...)`.

## 10. Normal and advanced API boundaries

| Situation | Choose |
|---|---|
| Normal iteration | `World::query` / `query_mut` |
| Occasional access by ID | `get` / `get_mut` |
| Repeated access to one component by many IDs | `accessor` / `accessor_mut` |
| Explicitly retain or reuse a query plan across Worlds | `PreparedQuery` |
| Install a reusable module | `Plugin` / `World::install` |
| Component types are known only at runtime | `sky_ecs::dynamic` |
| Explicit low-level Archetype / uninitialized construction | `sky_ecs::expert` |

When in doubt, prefer bundles, `World::query`, and `Commands`. These are the
normal application paths.

## 11. Further reading

- [Tutorial from first principles](TUTORIAL.md)
- [Progressive runnable examples](../crates/sky_ecs/examples/README.md)
- [Item-by-item Rustdoc API](https://docs.rs/sky_ecs)

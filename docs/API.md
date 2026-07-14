# Sky ECS API Guide

[中文](API_zh.md) · [Tutorial](TUTORIAL.md) · [Rustdoc](https://docs.rs/sky_ecs)

This guide maps common ECS tasks to the public API. Core ECS types are available
directly from `sky_ecs` and through `sky_ecs::ecs`; plugin types are exported
from `sky_ecs` and `sky_ecs::plugin`.

## API map

| Task | API |
|---|---|
| Own ECS state | `World` |
| Create entities | `World::spawn`, `World::spawn_batch` |
| Read or update one entity | `get`, `get_mut`, `has`, `insert`, `remove` |
| Iterate components | `query`, `query_mut` |
| Filter archetypes | `With`, `Without`, `Any` |
| Reuse an explicit query plan | `PreparedQuery` |
| Defer structural changes | `Commands`, `CommandBuffer` |
| Store singleton state | World resources, `Res`, `ResMut` |
| Run systems | `View`, `ParView`, stages, `tick` |
| Install modules | `Plugin`, `World::install` |
| Runtime-known component types | `sky_ecs::dynamic` |
| Low-level storage integration | `sky_ecs::expert` |

## World, entities, and components

Any `'static` Rust type can be a component. A tuple of components is a bundle.
`EntityId` is generational: after despawn, an old ID never names a newly reused
entity slot.

```rust
use sky_ecs::World;

struct Position(f32, f32);
struct Velocity(f32, f32);

let mut world = World::new();
let entity = world.spawn((Position(0.0, 0.0), Velocity(1.0, 2.0)));

assert!(world.contains(entity));
assert!(world.has::<Position>(entity));
world.get_mut::<Position>(entity).unwrap().0 += 1.0;
world.insert(entity, String::from("player"));
world.remove::<Velocity>(entity);
assert!(world.despawn(entity));
assert!(!world.contains(entity));
```

Use `spawn_batch` when entities share one bundle shape:

```rust
world.spawn_batch((0..10_000).map(|i| (Position(i as f32, 0.0), Velocity(1.0, 0.0))));
```

`clear` removes all entities but keeps resources. `entity_count` reports live
entities.

## Typed queries

Use `query` for read-only data and `query_mut` when any item is mutable.

```rust
world.query::<&Position>().for_each(|position| {
    std::hint::black_box(position);
});

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.0 += velocity.0;
        position.1 += velocity.1;
    });
```

Query items can be references, tuples, and optional references:

```rust
world
    .query::<(&Position, Option<&Velocity>)>()
    .for_each(|(position, velocity)| {
        let _ = (position, velocity);
    });
```

Useful traversal methods include `for_each`, `for_each_with_entity`,
`for_each_chunk`, `for_each_chunk_with_entities`, `count`, and `is_empty`.
Chunk methods expose aligned component slices for batch processing.

For wide named queries, derive `QueryData`:

```rust
use sky_ecs::QueryData;

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

world.query_mut::<Movement>().for_each(|item| {
    item.position.0 += item.velocity.0;
});
```

## Filters

Filters select archetypes at compile time and do not add values to the query
item.

```rust
use sky_ecs::{Any, With, Without};

struct Player;
struct Disabled;
struct Selected;

world
    .query::<&Position>()
    .filter::<(With<Player>, Without<Disabled>)>()
    .for_each(|position| { let _ = position; });

let visible = world
    .query::<&Position>()
    .filter::<Any<(With<Player>, With<Selected>)>>();
```

## Parallel iteration

Bound queries provide `par_for_each` and `par_for_each_chunk`. Small workloads
automatically stay sequential.

```rust
world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|(position, velocity)| {
        position.0 += velocity.0;
    });
```

In systems, use `ParView<Q>` when parallel methods are required; use `View<Q>`
for sequential work.

## Reusable plans and random access

`World::query` caches matching plans in the world and is the normal API.
`PreparedQuery<Q, F>` is useful when a system or extractor must explicitly own a
reusable plan or reuse it across worlds.

For a loop that repeatedly looks up components by `EntityId`, `accessor` and
`accessor_mut` bind component columns once. Use ordinary `get` and `get_mut` for
occasional access. Accessors borrow the world, so structural changes cannot
occur while they are alive.

## Resources

Resources are typed singleton values:

```rust
#[derive(Default)]
struct Score(u32);

world.insert_resource(Score::default());
world.get_resource_mut::<Score>().unwrap().0 += 10;
assert!(world.contains_resource::<Score>());
let score = world.remove_resource::<Score>().unwrap();
```

Systems request resources with `Res<T>` and `ResMut<T>`.

## Structural changes and commands

Direct `spawn`, `despawn`, `insert`, and `remove` require mutable world access.
Use `Commands` inside systems or an owned `CommandBuffer` outside the scheduler
to defer structural work.

```rust
use sky_ecs::CommandBuffer;

let entity = world.spawn((Position(0.0, 0.0),));
let mut commands = CommandBuffer::new();
commands.insert(entity, Velocity(1.0, 0.0));
commands.spawn((Position(5.0, 0.0),));
commands.apply(&mut world);
```

## Systems and stages

System parameters declare access. Compatible systems may run together;
conflicting systems retain a stable order.

```rust
use sky_ecs::{Res, Time, Update, View};

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.0 += velocity.0 * time.delta;
    });
}

world.stage(Update).add(movement);
world.tick_with_delta(1.0 / 60.0).unwrap();
world.shutdown();
```

Built-in order: `First`, `FixedUpdate`, `PreUpdate`, `Update`, `PostUpdate`,
`Last`. Configure fixed steps with `FixedStep`.

## Plugins and advanced APIs

Implement `Plugin` for a module that installs resources and systems, then call
`world.install(plugin)`. Duplicate plugin types are rejected.

Use `sky_ecs::dynamic` for tools, scripting, and reflection-driven code whose
component types are known only at runtime. Use `sky_ecs::expert` only for
explicit low-level archetype or uninitialized-spawn integration. Normal game
code should prefer typed bundles and queries.

## Further reading

- [Tutorial](TUTORIAL.md)
- [Progressive examples](../crates/sky_ecs/examples/README.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)

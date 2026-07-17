# Sky ECS Tutorial

[中文](TUTORIAL_zh.md) · [API guide](API.md)

This tutorial builds a small movement simulation and then turns it into a
scheduled ECS application.

## 1. Create a project

```toml
[dependencies]
sky_ecs = "0.1.3"
```

Create `src/main.rs` and import `World`:

```rust
use sky_ecs::World;
```

## 2. Define components

Components are ordinary Rust types. Keep data separate by responsibility.

```rust
#[derive(Clone, Copy, Debug)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy, Debug)]
struct Velocity { x: f32, y: f32 }

#[derive(Clone, Copy, Debug)]
struct Enemy;
```

Marker components such as `Enemy` contain no data but are useful for filters.

## 3. Spawn entities

```rust
let mut world = World::new();

let player = world.spawn((
    Position { x: 0.0, y: 0.0 },
    Velocity { x: 2.0, y: 0.0 },
));

world.spawn_batch((0..1_000).map(|i| (
    Position { x: i as f32, y: 20.0 },
    Velocity { x: -1.0, y: 0.0 },
    Enemy,
)));

assert!(world.contains(player));
assert_eq!(world.entity_count(), 1_001);
```

`spawn` returns an `EntityId`. Store it when gameplay needs to address that
specific entity. Use queries for bulk work.

## 4. Move every matching entity

```rust
let dt = 1.0 / 60.0;

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.x += velocity.x * dt;
        position.y += velocity.y * dt;
    });
```

The query only visits entities containing both components. Component access is
checked by Rust's type system: overlapping mutable access is rejected.

## 5. Filter and inspect

```rust
use sky_ecs::With;

let enemies = world.query::<&Position>().filter::<With<Enemy>>();
println!("enemy count: {}", enemies.count());

enemies.for_each_with_entity(|entity, position| {
    if position.x < 0.0 {
        println!("{entity:?} left the map");
    }
});
```

Use `Without<T>` to exclude a marker and `Any<(...)>` for OR filters. Use
`Option<&T>` when a component is optional data rather than a filter condition.

## 6. Defer structural changes

Do not add, remove, spawn, or despawn entities from inside an active query.
Collect those operations in a `CommandBuffer`:

```rust
use sky_ecs::CommandBuffer;

let mut commands = CommandBuffer::new();
commands.despawn(player);
commands.spawn((
    Position { x: 5.0, y: 5.0 },
    Velocity { x: 0.0, y: 1.0 },
));
commands.apply(&mut world);
```

Systems use the borrowed `Commands` parameter instead.

## 7. Add resources and systems

Resources hold world-wide singleton state. System parameters describe data
access, allowing the scheduler to order conflicting systems safely.

```rust
use sky_ecs::{Res, ResMut, Time, Update, View};

#[derive(Default)]
struct FrameCount(u64);

fn movement(
    bodies: View<(&mut Position, &Velocity)>,
    time: Res<Time>,
) {
    bodies.for_each(|(position, velocity)| {
        position.x += velocity.x * time.delta;
        position.y += velocity.y * time.delta;
    });
}

fn count_frame(mut frame: ResMut<FrameCount>) {
    frame.0 += 1;
}

world.insert_resource(FrameCount::default());
world.stage(Update).add(movement).add(count_frame);

for _ in 0..60 {
    world.tick_with_delta(1.0 / 60.0).unwrap();
}

world.shutdown();
assert_eq!(world.get_resource::<FrameCount>().unwrap().0, 60);
```

Use `FixedUpdate` with `FixedStep::hz(...)` for fixed-rate simulation. Use
`ParView<Q>` and `par_for_each` when a system has enough work to benefit from
parallel execution.

## 8. Choose the right access pattern

- Use `World::query` / `query_mut` for normal iteration.
- Use chunk iteration when slices help batching or vectorization.
- Use `PreparedQuery` only when code must explicitly own a reusable plan.
- Use `get` / `get_mut` for occasional access by `EntityId`.
- Batch or defer structural changes instead of interleaving them with queries.

## Run the repository examples

```bash
cargo run -p sky_ecs --example step_01_world
cargo run -p sky_ecs --example step_02_queries
cargo run -p sky_ecs --example step_03_batches_and_chunks
cargo run -p sky_ecs --example step_04_commands
cargo run -p sky_ecs --example step_05_systems
cargo run -p sky_ecs --example step_06_parallel
cargo run -p sky_ecs --example step_07_tiny_defense
cargo run -p sky_ecs --example step_08_dynamic
cargo run -p sky_ecs --example step_09_plugin
```

Steps 01-07 are the core path; steps 08-09 cover advanced APIs. See the
[example index](../crates/sky_ecs/examples/README.md), then continue with the [API guide](API.md) or the
[generated Rust documentation](https://docs.rs/sky_ecs).

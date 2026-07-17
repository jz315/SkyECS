# Sky ECS

Sky ECS is a typed, chunk-based Entity Component System for Rust. It is the ECS
used by [SkyEngine](https://github.com/jz315/SkyEngine), but the crate can be
used independently.

## Documentation

- [Tutorial](https://github.com/jz315/SkyECS/blob/main/docs/TUTORIAL.md) / [中文](https://github.com/jz315/SkyECS/blob/main/docs/TUTORIAL_zh.md)
- [API guide](https://github.com/jz315/SkyECS/blob/main/docs/API.md) / [中文](https://github.com/jz315/SkyECS/blob/main/docs/API_zh.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)
- [Progressive examples](https://github.com/jz315/SkyECS/blob/main/crates/sky_ecs/examples/README.md) / [中文](https://github.com/jz315/SkyECS/blob/main/crates/sky_ecs/examples/README_zh.md)

## Features

- Chunk-columnar archetype storage
- Generational entity identifiers
- Bundle-based spawning and cached structural transitions
- World-bound typed queries and reusable `PreparedQuery`
- Optional query parameters and `With`, `Without`, and `Any` filters
- Entity and chunk iteration, with sequential and parallel variants
- Named query items through `#[derive(QueryData)]`
- Typed resources, systems, stages, fixed steps, and deferred commands
- Runtime-typed access under `sky_ecs::dynamic`
- Low-level storage APIs under `sky_ecs::expert`

## Usage

```toml
[dependencies]
sky_ecs = "0.1.3"
```

```rust
use sky_ecs::World;

#[derive(Clone, Copy)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy)]
struct Velocity { x: f32, y: f32 }

fn main() {
    let mut world = World::new();

    world.spawn_batch((0..10_000).map(|i| (
        Position { x: i as f32, y: 0.0 },
        Velocity { x: 80.0, y: 30.0 },
    )));

    world
        .query_mut::<(&mut Position, &Velocity)>()
        .for_each(|(position, velocity)| {
            position.x += velocity.x / 60.0;
            position.y += velocity.y / 60.0;
        });
}
```

For repeated random access to one component type, bind a component accessor to
the world before entering the hot loop:

```rust
let positions = world.accessor::<Position>();

for entity in entities {
    if let Some(position) = positions.get(entity) {
        println!("({}, {})", position.x, position.y);
    }
}
```

The accessor resolves matching component columns once and keeps a read-only
borrow of the world while it is alive. Use `World::get` for occasional lookups,
or when the world must be structurally changed between accesses.

Repeated random updates use an exclusive accessor:

```rust
let mut positions = world.accessor_mut::<Position>();

for entity in entities {
    if let Some(position) = positions.get_mut(entity) {
        position.x += 1.0;
    }
}
```

## Queries

Queries are created from a `World`. Matching archetype plans are cached by the
world and refreshed after structural changes.

```rust
use sky_ecs::{With, Without};

struct Player;
struct Disabled;

let query = world
    .query::<(&Position, Option<&Velocity>)>()
    .filter::<(With<Player>, Without<Disabled>)>();
```

Use `query_mut` when a query contains mutable component references. Use
`PreparedQuery` when the query plan needs to be stored explicitly or reused
across worlds.

Named query items are available for wider queries:

```rust
use sky_ecs::QueryData;

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

world.query_mut::<Movement>().for_each(|body| {
    body.position.x += body.velocity.x;
    body.position.y += body.velocity.y;
});
```

Parallel iteration uses the same query types:

```rust
world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });
```

Small workloads fall back to sequential iteration. `par_for_each_chunk` is
available when the inner loop benefits from component slices.

## Systems

System access is inferred from typed parameters. Compatible systems may run in
parallel, while conflicting systems keep registration order.

```rust
use sky_ecs::{Res, Time, Update, View, World};

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.x += velocity.x * time.delta;
        position.y += velocity.y * time.delta;
    });
}

let mut world = World::new();
world.stage(Update).add(movement);
world.tick_with_delta(1.0 / 60.0).unwrap();
```

Structural changes made by systems can be deferred through `Commands`. Outside
the scheduler, use an owned `CommandBuffer`.

## Benchmarks

The repository includes a Criterion comparison of six ECS implementations.
Compare-ECS limits conclusions to its single-threaded public-API workloads,
uses each implementation's fastest suitable reusable query, view, or accessor
state, and validates every adapter before measurement. Results are
machine-specific. The benchmark guide records current-protocol local
measurements from 2026-07-17, including a Clang/LLVM 22.1.2 remeasurement of
the Flecs column; see it for workload classifications, compiler configuration,
and measurement boundaries. Prepared random access excludes
preparation cost and cache memory, while scenarios and diagnostics are reported
separately from comparable workloads.

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

The methodology and recorded results are kept in the
[benchmark guide](https://github.com/jz315/SkyECS/blob/main/benches/BENCHMARKS.md).
Internal Sky ECS benchmarks are kept separate and run with `cargo bench`.

## API overview

| Need | API |
|---|---|
| Read-only query | `World::query` |
| Mutable query | `World::query_mut` |
| Repeated random component access | `World::accessor`, `World::accessor_mut` |
| Explicit reusable query plan | `PreparedQuery` |
| Parallel entity iteration | `par_for_each` |
| Parallel chunk iteration | `par_for_each_chunk` |
| Archetype filters | `With<T>`, `Without<T>`, `Any<(...)>` |
| Deferred structural changes | `Commands`, `CommandBuffer` |
| Runtime-known component types | `sky_ecs::dynamic` |
| Low-level storage access | `sky_ecs::expert` |

Typed component tuples for bundles, queries, and filters support up to 16
entries. A single internal archetype can store up to 32 distinct component
types; that limit applies per archetype, not per `World`.

Sky ECS requires Rust 1.85 or newer. The crate is currently versioned as `0.x`.

## License

MIT. See [LICENSE](https://github.com/jz315/SkyECS/blob/main/LICENSE).

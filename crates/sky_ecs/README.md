# Sky ECS

Sky ECS is a typed, chunk-based Entity Component System for Rust. It is the ECS
used by [SkyEngine](https://github.com/jz315/SkyEngine), but the crate can be
used independently.

## Documentation

- [Tutorial](https://github.com/jz315/SkyECS/blob/main/docs/TUTORIAL.md) / [中文](https://github.com/jz315/SkyECS/blob/main/docs/TUTORIAL_zh.md)
- [API reference](https://github.com/jz315/SkyECS/blob/main/docs/API.md) / [中文](https://github.com/jz315/SkyECS/blob/main/docs/API_zh.md)
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
sky_ecs = "0.3.0"
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
        .for_each(|position, velocity| {
            position.x += velocity.x / 60.0;
            position.y += velocity.y / 60.0;
        });
}
```

For repeated random access to one component type, bind an entity accessor to
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

When the same component is accessed by changing entity IDs across many frames,
retain its route table and bind it for each access phase:

```rust
let mut prepared_positions = sky_ecs::PreparedEntityAccessor::<Position>::new();

for frame in frames {
    let positions = prepared_positions.bind(&world);
    for entity in frame.targets {
        use_position(positions.get(entity));
    }
}
```

Binding reacquires live entity records but reuses component column routes until
`Position` column bases or the route-table shape changes. Backing changes in
unrelated archetypes do not rebuild this cache.

When the same fixed entity order is reused, prepare its component addresses
once and iterate the resulting direct access plan:

```rust
let positions = world.prepare_access::<Position>(&entities).unwrap();

for position in positions.iter() {
    println!("({}, {})", position.x, position.y);
}
```

Preparation fails if an entity is stale or lacks the requested component. A
mutable plan is available through `prepare_access_mut`; it additionally rejects
duplicate entities so its iterator can safely yield disjoint mutable references.

Repeated random updates use an exclusive accessor:

```rust
let mut positions = world.accessor_mut::<Position>();

for entity in entities {
    if let Some(position) = positions.get_mut(entity) {
        position.x += 1.0;
    }
}
```

When one entity lookup needs several components, retain a tuple-capable view
and bind it once per access phase:

```rust
let mut ai = sky_ecs::PreparedEntityView::<(&TargetSlot, &mut Cooldown)>::new();
let mut view = ai.bind_mut(&mut world);

for entity in entities {
    if let Some((target, cooldown)) = view.get_mut(entity) {
        cooldown.0 = cooldown.0.saturating_sub(1);
        use_target(target);
    }
}
```

Binding refreshes component pointers when a queried component's column bases or
the route-table shape changes, while reusing the prepared allocations. Unrelated
archetype churn does not rebuild the view. Each lookup validates the entity
route once and returns the complete tuple.

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

Named query declarations are available for wider queries. Iteration callbacks
receive their fields as separate arguments in declaration order:

```rust
use sky_ecs::QueryData;

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

world.query_mut::<Movement>().for_each(|position, velocity| {
    position.x += velocity.x;
    position.y += velocity.y;
});
```

Parallel iteration uses the same query types:

```rust
world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|position, velocity| {
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
use sky_ecs::{EntityId, EntityView, Res, Time, Update, View, World};

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|position, velocity| {
        position.x += velocity.x * time.delta;
        position.y += velocity.y * time.delta;
    });
}

fn update_selected(mut bodies: EntityView<(&Position, &mut Velocity)>, entity: Res<EntityId>) {
    if let Some((_position, velocity)) = bodies.get_mut(*entity) {
        velocity.x *= 0.5;
        velocity.y *= 0.5;
    }
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
machine-specific. The benchmark guide records workload classifications,
compiler configuration, and measurement boundaries. Entity-ID random access
starts from an ID in every adapter; fixed-sequence plan build, steady traversal,
payload, and amortization are a separate Scenario. Diagnostics and native
capability scenarios are reported separately from Comparable workloads.

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

The methodology and recorded results are kept in the
[benchmark guide](https://github.com/jz315/SkyECS/blob/main/benches/BENCHMARKS.md).
Internal Sky ECS API candidates are kept in this crate's `benches/` directory
and run locally with `cargo bench`; the formal comparison contains only the
selected paths and never chooses a winner on GitHub shared runners.

## API overview

| Need | API |
|---|---|
| Read-only query | `World::query` |
| Mutable query | `World::query_mut` |
| Repeated access with arbitrary entity IDs | `World::accessor`, `World::accessor_mut` |
| Reused fixed entity sequence | `World::prepare_access`, `World::prepare_access_mut` |
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

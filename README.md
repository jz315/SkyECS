# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

**A high-performance, typed, chunk-based Entity Component System for Rust.**


[中文](README_zh.md)

Sky ECS is the standalone ECS used by
[SkyEngine](https://github.com/jz315/SkyEngine). It provides archetype storage,
typed queries, parallel iteration, resources, deferred commands, and a small
typed system scheduler.

The repository includes a reproducible single-threaded comparison against
`hecs`, `bevy_ecs`, `flecs_ecs`, `freecs`, and `shipyard`. Results are scoped to
the measured safe public APIs and workloads rather than treated as a universal
ECS ranking.

## Features

- Chunk-columnar archetype storage
- Generational entity identifiers and cached structural transitions
- Bundle-based `spawn` and `spawn_batch`
- World-cached typed queries and reusable `PreparedQuery`
- Entity and chunk iteration, including parallel variants
- Optional query parameters and compile-time filters
- Typed resources, systems, stages, fixed steps, and deferred commands
- Runtime-typed and low-level APIs under `dynamic` and `expert`

## Quick start

```toml
[dependencies]
sky_ecs = "0.1.2"
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

Use the same query with `par_for_each` or `par_for_each_chunk` when the workload
is large enough to benefit from parallel execution.

The crate-level guide is in
[`crates/sky_ecs/README.md`](crates/sky_ecs/README.md).

## Benchmarks

The benchmark compares Sky ECS, hecs, Bevy ECS, Flecs, FreeCS, and Shipyard
through functionality shared by all six libraries under the same environment.

In the recorded results, Sky has the lowest times for bulk insertion, single
insertion, spawn/despawn, and the mixed-frame scenario, while its iteration
performance is effectively tied with Flecs.

Key results:

| Workload | Sky | hecs | Bevy | Flecs | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **146.70 µs** | 242.59 µs | 287.62 µs | N/A | 261.05 µs | 157.98 µs |
| Iterate 10k | **4.96 µs** | 5.10 µs | 7.69 µs | 5.15 µs | 7.78 µs | 11.03 µs |
| Iterate 100k | **52.31 µs** | 55.08 µs | 80.75 µs | **52.01 µs** | 79.49 µs | 114.18 µs |
| Spawn/despawn 1k | **19.57 µs** | 24.51 µs | 63.54 µs | 157.58 µs | 72.28 µs | 59.71 µs |
| Mixed frame | **181.68 µs** | 195.09 µs | 238.81 µs | 223.63 µs | 208.04 µs | 200.05 µs |

See the [benchmark documentation](benches/BENCHMARKS.md) for all 19 workloads,
the test environment, dependency versions, and measurement notes.

## Workspace

```text
crates/sky_ecs/          ECS runtime, examples, and internal benchmarks
crates/sky_ecs_derive/   QueryData and StageLabel derive macros
crates/sky_type/         Runtime type identity and layout metadata
tools/ecs-comparison/    Cross-engine benchmark suite
```

Sky ECS requires Rust 1.85 or newer.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT.

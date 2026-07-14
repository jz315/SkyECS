# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

**A high-performance, typed, chunk-based Entity Component System for Rust.**

[中文](README_zh.md)

Sky ECS is a very fast Entity Component System (ECS) library for Rust, with
leading results across multiple performance benchmarks.

> Sky ECS is also the built-in ECS component of the
> [SkyEngine](https://github.com/jz315/SkyEngine) game engine.

In cross-library comparisons against `hecs`, `bevy_ecs`, `flecs_ecs`, `freecs`,
and `shipyard`, Sky ECS records the lowest times for **bulk insertion,
entity creation/destruction, and mixed-frame** workloads.

## Features

- Extreme performance: an Archetype architecture and deep core optimizations
  provide exceptionally fast execution.
- Native parallelism: built-in multithreading makes full use of multi-core CPUs.
- Elegant and easy to use: intuitive APIs let developers focus on application
  and game logic.
- Dynamic extensibility: alongside the typed API, a complete dynamic API supports
  runtime reflection and integration with languages such as C# and scripting
  languages.

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

For larger workloads, replace serial iteration with `par_for_each` or
`par_for_each_chunk` to take advantage of multiple CPU cores.

See the [crate guide](crates/sky_ecs/README.md),
[API documentation](https://docs.rs/sky_ecs), and
[`examples/`](crates/sky_ecs/examples/) for more usage examples.

## Benchmarks

The benchmark uses functionality and public APIs shared by all six libraries to
compare Sky ECS, hecs, Bevy ECS, Flecs, FreeCS, and Shipyard in the same
environment.

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

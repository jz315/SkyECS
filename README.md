# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[中文](README_zh.md)

---

**A high-performance, typed, chunk-based Entity Component System for Rust.**



Sky ECS is a very fast Entity Component System (ECS) library for Rust, with
leading results across multiple performance benchmarks.

> Sky ECS is also the built-in ECS component of the
> [SkyEngine](https://github.com/jz315/SkyEngine) game engine.

In cross-implementation comparisons against `hecs`, `bevy_ecs`, `Flecs`,
`freecs`, and `shipyard`, Sky ECS records the lowest times for **bulk insertion,
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

## Documentation

- [Tutorial](docs/TUTORIAL.md)
- [API guide](docs/API.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)
- [Progressive examples](crates/sky_ecs/examples/README.md)

## Benchmarks

The benchmark uses shared workloads and public APIs to compare seven ECS
implementations in the same environment.

On the recorded machine, Sky records the lowest median for bulk and
single insertion, 10k and 100k prepared iteration, spawn/despawn, and the
mixed-frame scenario.

Key results:

| Workload | Sky | hecs | Bevy | Flecs | Flecs C++ | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **145.19 µs** | 202.71 µs | 292.65 µs | 236.92 µs | 208.84 µs | 265.58 µs | 203.98 µs |
| Prepared iteration 10k | **5.21 µs** | 5.37 µs | 8.08 µs | 5.46 µs | 6.33 µs | 6.88 µs | 11.33 µs |
| Prepared iteration 100k | **55.76 µs** | 56.82 µs | 85.05 µs | 57.75 µs | 68.06 µs | 70.41 µs | 116.54 µs |
| Spawn/despawn 1k | **16.84 µs** | 20.56 µs | 61.77 µs | 38.42 µs | 23.38 µs | 93.52 µs | 62.40 µs |
| Mixed frame | **183.61 µs** | 260.78 µs | 273.46 µs | 207.86 µs | 219.23 µs | 274.59 µs | 207.08 µs |

See the [benchmark documentation](benches/BENCHMARKS.md) for the complete workload list,
Flecs audit provenance, test environment, dependency versions, and measurement notes.


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

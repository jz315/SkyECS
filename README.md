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

Compare-ECS covers `hecs`, `bevy_ecs`, Flecs, `freecs`, and `shipyard` through
shared workloads and public APIs.

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
sky_ecs = "0.2.0"
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
- [API reference](docs/API.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)
- [Progressive examples](crates/sky_ecs/examples/README.md)

## Benchmarks

[GitHub Actions run 30179149116](https://github.com/jz315/SkyECS/actions/runs/30179149116)

| Test | Scale / Mode | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| Entity construction | Individual 10K | **303.595 µs** | 579.261 µs† | 674.912 µs | 587.618 µs | 926.500 µs† | 1.923 ms |
| Entity construction | Native bulk 10K | 98.401 µs† | **13.407 µs** | 381.477 µs | 90.245 µs† | 294.031 µs | 186.128 µs† |
| Entity operations | Spawn/despawn 1K | 53.246 µs | **47.814 µs** | 103.659 µs | 63.135 µs | 111.841 µs | 165.636 µs |
| Entity operations | Add/remove component 1K | 108.509 µs | 109.488 µs | 161.058 µs | 142.036 µs | 222.387 µs | **74.846 µs** |
| EntityId random access | Hot 10K | 17.827 µs | **16.669 µs** | 45.692 µs | 41.246 µs | 27.244 µs | 20.631 µs |
| EntityId random access | Warm 100K | **300.135 µs** | 301.431 µs | 871.652 µs | 710.293 µs | 486.573 µs | 426.576 µs |
| Prepared iteration | 10K | 8.173 µs | 8.188 µs | 9.674 µs | **8.100 µs** | 13.144 µs | 18.206 µs |
| Prepared iteration | 100K | **81.144 µs** | 81.885 µs | 99.576 µs | 82.122 µs | 134.350 µs | 184.842 µs |
| Prepared iteration | 1M | 832.267 µs | 855.464 µs | 1.112 ms† | **829.999 µs** | 1.360 ms | 1.865 ms |
| Fragmented iteration | 26 × 400 | 1.160 µs | 4.459 µs | 7.750 µs | 1.265 µs | 988.421 ns | **923.137 ns** |
| Gameplay | Full frame | **121.092 µs** | 147.955 µs | 211.859 µs | 140.657 µs | 189.282 µs | 328.011 µs |

Lower is faster; `†` marks a noisy shared-runner result. See the
[benchmark documentation](benches/BENCHMARKS.md) for the complete 37-row
report, workload contract, and reproduction commands.


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

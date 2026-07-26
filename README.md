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

[GitHub Actions run 30210139416](https://github.com/jz315/SkyECS/actions/runs/30210139416)

| Test | Scale / Mode | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| Entity construction | Individual 10K | **276.147 µs** | 580.473 µs | 812.098 µs | 746.055 µs | 883.430 µs | 1.216 ms |
| Entity construction | Bulk construction 10K | 97.557 µs† | **85.942 µs**† | 522.206 µs† | 113.279 µs† | 327.816 µs | 478.557 µs |
| Entity operations | Spawn/despawn 1K | 52.233 µs | **46.841 µs** | 104.725 µs | 69.633 µs | 111.703 µs | 112.621 µs |
| Entity operations | Add/remove component 1K | 107.321 µs | 109.014 µs | 164.131 µs | 135.085 µs | 219.504 µs | **51.248 µs** |
| EntityId random access | Hot 10K | 16.628 µs | **15.982 µs** | 44.384 µs | 37.963 µs | 23.440 µs | 20.246 µs |
| EntityId random access | Warm 100K | 304.531 µs | **299.879 µs** | 901.731 µs | 694.855 µs | 453.338 µs | 457.975 µs |
| Prepared iteration | 10K | 7.819 µs | 7.773 µs | 9.327 µs | **7.471 µs** | 12.136 µs | 17.657 µs |
| Prepared iteration | 100K | 77.263 µs | 78.032 µs | 93.822 µs | **75.782 µs** | 119.623 µs | 176.043 µs |
| Prepared iteration | 1M | **910.881 µs**† | 954.929 µs† | 1.086 ms† | 971.924 µs† | 1.258 ms | 1.794 ms |
| Fragmented iteration | 26 × 400 | 1.051 µs | 4.254 µs | 6.844 µs | 1.169 µs | 859.694 ns | **812.176 ns** |
| Gameplay | Full frame | **121.191 µs** | 146.026 µs | 212.210 µs | 138.926 µs | 183.532 µs | 326.138 µs |

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

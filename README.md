# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[中文](README_zh.md)

---

**A high-performance, archetype-based Entity Component System for Rust.**

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

For larger workloads, replace serial iteration with `par_for_each` or
`par_for_each_chunk` to take advantage of multiple CPU cores.

## Documentation

- [Tutorial](docs/TUTORIAL.md)
- [API reference](docs/API.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)
- [Progressive examples](crates/sky_ecs/examples/README.md)

## Benchmarks

[GitHub Actions run 30705936563](https://github.com/jz315/SkyECS/actions/runs/30705936563)

| Test | Scale / Mode | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| Entity construction | Individual 10K | **275.942 µs**† | 552.600 µs† | 771.155 µs† | 698.128 µs | 829.703 µs† | 1.161 ms |
| Entity construction | Bulk construction 10K | **42.712 µs**† | 63.621 µs† | 504.789 µs | 84.409 µs | 323.161 µs† | 466.833 µs |
| Entity operations | Spawn/despawn 1K | **41.540 µs** | 44.792 µs | 102.800 µs | 68.277 µs | 110.843 µs | 112.659 µs |
| Entity operations | Add/remove component 1K | 88.204 µs | 109.202 µs | 161.564 µs | 119.973 µs | 224.711 µs | **52.114 µs** |
| EntityId random access | Hot 10K | 16.445 µs | **16.147 µs** | 44.372 µs | 37.982 µs | 23.881 µs | 20.070 µs |
| EntityId random access | Warm 100K | 303.143 µs | **293.978 µs** | 858.468 µs | 670.284 µs | 445.330 µs | 439.425 µs |
| Prepared iteration | 10K | 7.755 µs | 7.770 µs | 9.440 µs | **7.690 µs** | 11.819 µs | 17.336 µs |
| Prepared iteration | 100K | 77.338 µs | 78.275 µs | 94.956 µs | **75.214 µs** | 119.823 µs | 173.370 µs |
| Prepared iteration | 1M | 825.249 µs† | 850.049 µs† | 954.499 µs† | **796.771 µs**† | 1.194 ms | 1.772 ms |
| Fragmented iteration | 26 × 400 | 1.045 µs | 6.854 µs | 6.840 µs | 1.125 µs | 860.278 ns | **811.752 ns** |
| Gameplay | Full frame | **113.920 µs** | 138.687 µs | 201.104 µs | 134.947 µs | 177.867 µs | 311.203 µs |

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

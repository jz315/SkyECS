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

The numeric rows below are a traceable historical snapshot from public
[GitHub Actions run #29695552048](https://github.com/jz315/SkyECS/actions/runs/29695552048)
at commit `e47f48163759f2e0438bcb89504908749999a416`. The old Mixed frame has been
retired: matrix inversion dominated it, so it did not represent ECS behavior.
Its replacement is a deterministic 65,536-entity, 256-frame gameplay trace with
real status and projectile lifetimes. New gameplay and best-native-bulk numbers
will appear here only after the updated four-rotation public workflow completes.
Sky API candidates are measured locally in `crates/sky_ecs/benches`; the formal
Compare-ECS target and GitHub workflow contain only the selected paths and never
choose an API on a shared runner.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Legacy row-batch insert 10k (retired) | 120.93 µs | 352.11 µs | 440.19 µs | **110.41 µs** | 278.08 µs | 166.75 µs |
| Prepared iteration 10k | 8.12 µs | 7.83 µs | 9.35 µs | **7.69 µs** | 11.96 µs | 17.29 µs |
| Prepared iteration 100k | 81.21 µs | 78.88 µs | 93.65 µs | **77.62 µs** | 120.08 µs | 174.30 µs |
| Spawn/random despawn 1k | **45.37 µs** | 46.34 µs | 103.05 µs | 67.47 µs | 112.41 µs | 107.42 µs |
| Gameplay frame (new canonical) | pending public rerun | pending | pending | pending | pending | pending |

See the [benchmark documentation](benches/BENCHMARKS.md) for the complete workload
list, all recorded rows, environment, compiler configuration, and methodology.


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

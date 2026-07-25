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

The formal [Compare-ECS workflow](https://github.com/jz315/SkyECS/actions/workflows/benchmarks.yml)
measures Sky, hecs, Bevy ECS, Flecs C, FreeCS, and Shipyard through equivalent
public APIs. Each report has a fixed 37-row layout:

- 10 Comparable rows
- 20 Random Fragmentation rows
- 6 Gameplay Scenario rows: full frame plus five phases
- 1 Diagnostic row

Fixed Sequence Access and API-candidate selection are local experiments and do
not run in the GitHub performance workflow. See the
[benchmark documentation](benches/BENCHMARKS.md) for the complete workload
contract, report format, and reproduction commands.


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

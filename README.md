# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[中文](README_zh.md)

---

**A high-performance, typed, chunk-based Entity Component System for Rust.**

**The current benchmark has issues and the results are being reviewed.**

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

For larger workloads, replace serial iteration with `par_for_each` or
`par_for_each_chunk` to take advantage of multiple CPU cores.

Typed bundle, query, and filter tuples support up to 16 entries. A single
archetype can contain up to 32 distinct component types; this is a per-archetype
storage limit, not a limit on the component types available to a `World`.

## Documentation

- [Tutorial](docs/TUTORIAL.md)
- [API guide](docs/API.md)
- [Generated Rust API documentation](https://docs.rs/sky_ecs)
- [Progressive examples](crates/sky_ecs/examples/README.md)

## Benchmarks

The table records current-protocol local measurements from 2026-07-17. The
Flecs column was remeasured with Clang/LLVM 22.1.2 after the native compiler
audit. Spawn/random despawn was remeasured for all adapters after adopting a
deterministic shuffled deletion order.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **121.72 µs** | 205.14 µs | 307.23 µs | 237.70 µs | 263.85 µs | 282.87 µs |
| Prepared iteration 10k | 5.31 µs | 5.35 µs | 7.93 µs | **5.14 µs** | 6.88 µs | 11.48 µs |
| Prepared iteration 100k | 57.94 µs | 58.48 µs | 89.97 µs | **53.72 µs** | 70.77 µs | 119.94 µs |
| Spawn/random despawn 1k | 24.76 µs | **22.93 µs** | 79.59 µs | 39.37 µs | 108.85 µs | 64.29 µs |
| Mixed frame | 220.18 µs | **218.61 µs** | 254.18 µs | 290.92 µs | 312.64 µs | 287.53 µs |

Prepared random access measures only the prepared lookup hot path; preparation
cost and cache memory are excluded. Mixed frame is a scenario and heavy compute
is a diagnostic, so neither is used for an overall speed claim.

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

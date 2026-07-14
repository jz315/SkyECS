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

Compare-ECS v2 uses each ECS's recommended reusable safe query or view state,
isolates construction and teardown, validates every adapter, and measures hot,
warm, and cold random-access working sets.

Six-rotation cross-run medians recorded on 2026-07-14 on an i7-12700F:

Bold marks the lowest median, plus Sky results within 1% of the lowest.

| Workload | Sky | hecs | Bevy | Flecs | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **146.707 µs** | 242.587 µs | 287.617 µs | N/A | 261.048 µs | 157.979 µs |
| Single insert 10k | **207.656 µs** | 490.967 µs | 654.194 µs | 3.433 ms | 882.560 µs | 732.709 µs |
| Steady iteration 10k | **4.961 µs** | 5.096 µs | 7.693 µs | 5.146 µs | 7.784 µs | 11.029 µs |
| Steady iteration 10k × 32 | **158.364 µs** | 165.264 µs | 241.443 µs | 160.181 µs | 243.145 µs | 315.386 µs |
| Steady iteration 100k | **52.308 µs** | 55.084 µs | 80.748 µs | **52.013 µs** | 79.492 µs | 114.182 µs |
| Fragmented iteration 26 × 400 | 0.711 µs | 2.507 µs | 5.712 µs | 1.030 µs | 4.774 µs | **0.522 µs** |
| Random access, hot 10k | 15.080 µs | 128.483 µs | 35.095 µs | 314.902 µs | 14.632 µs | **10.157 µs** |
| Random access, warm 100k | 221.952 µs | 1.291 ms | 511.700 µs | 3.214 ms | 224.437 µs | **154.510 µs** |
| Random access, cold 1m | 5.624 ms | 15.090 ms | 16.485 ms | 51.450 ms | 5.737 ms | **3.759 ms** |
| Spawn/despawn 1k | **19.573 µs** | 24.510 µs | 63.543 µs | 157.581 µs | 72.275 µs | 59.710 µs |
| Add/remove component 1k | 45.521 µs | 57.667 µs | 83.604 µs | 118.230 µs | 98.812 µs | **25.174 µs** |
| Diagnostic heavy compute | **1.871 ms** | **1.865 ms** | 1.870 ms | 1.873 ms | 1.871 ms | 1.870 ms |
| Mixed frame | **181.680 µs** | 195.088 µs | 238.806 µs | 223.634 µs | 208.039 µs | 200.046 µs |
| Mixed phase: movement | **12.596 µs** | 13.125 µs | 18.948 µs | **12.550 µs** | 18.983 µs | 26.415 µs |
| Mixed phase: health × 8 | **5.274 µs** | 15.234 µs | 36.988 µs | 6.260 µs | 35.346 µs | 54.606 µs |
| Mixed phase: heavy | **151.747 µs** | 155.301 µs | 153.572 µs | **151.016 µs** | 152.090 µs | 151.769 µs |
| Mixed phase: random access | 2.980 µs | 6.827 µs | 1.889 µs | 16.154 µs | **0.615 µs** | 0.649 µs |
| Mixed phase: structural churn | 10.869 µs | 14.916 µs | 57.986 µs | 30.062 µs | 23.769 µs | **6.382 µs** |
| Mixed phase: spawn/despawn × 32 | **38.367 µs** | 51.285 µs | 541.702 µs | 320.296 µs | 139.507 µs | 147.644 µs |

The health and spawn/despawn phase rows repeat their operation to reduce timing
noise. Divide those totals by 8 and 32 respectively for a single-frame phase
estimate; isolated phase estimates do not exactly sum to the complete frame.

Sky leads the construction, spawn/despawn, and mixed-frame cases in this
single-threaded snapshot and remains in the leading iteration group. Shipyard
leads prepared random access and add/remove transitions. These results describe
the tested workloads, not every ECS use case.

Run one development pass or the six-order publication protocol with:

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

Methodology and measurement policy are documented in
[`benches/BENCHMARKS.md`](benches/BENCHMARKS.md).

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

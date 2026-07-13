# Sky ECS

**A high-performance, typed, chunk-based Entity Component System for Rust.**

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Sky ECS is designed for hot iteration loops, large worlds, predictable frame
time, and parallel systems without giving up a direct Rust API.

> **Fastest in our fair public-API suite.** In the recorded Criterion snapshot,
> Sky leads the tested `hecs`, `bevy_ecs`, and `flecs_ecs` implementations in
> simple iteration, bulk insertion, and mixed-frame simulation. Results depend
> on workload, hardware, compiler, and revision, so the complete reproducible
> comparison suite ships in this repository.

## Highlights

- Chunk-columnar archetype storage
- World-cached typed query plans
- Entity and chunk iteration
- Parallel queries with cached stripe jobs and automatic serial fallback
- Optional query parameters and compile-time archetype filters
- Named `#[derive(QueryData)]` query items
- Generational entities and cached structural transitions
- Typed resources, systems, stages, fixed steps, and deferred commands
- Safe runtime-typed APIs for tools and explicit expert escape hatches

## Quick start

```toml
[dependencies]
sky_ecs = "0.1.1"
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

Use the same query shape with `par_for_each` or `par_for_each_chunk` when the
workload is large enough to benefit from Rayon parallelism.

The detailed crate guide is in
[`crates/sky_ecs/README.md`](crates/sky_ecs/README.md).

## Benchmarks

The canonical comparison uses only safe public APIs available in Sky, `hecs`,
Bevy ECS, and Flecs, with query/prepared state created outside the timed loop.

```bash
cargo compare-ecs
cargo compare-ecs -- fair_iteration/simple/sky --exact
```

Internal regression benchmarks are separate:

```bash
cargo bench -p sky_ecs
```

See the [benchmark methodology](benches/BENCHMARKS.md) and its
[Chinese version](benches/BENCHMARKS_CN.md). Historical numbers are evidence
for a particular machine and revision, not a permanent universal guarantee.

## Workspace

```text
crates/sky_ecs/          ECS runtime and public API
crates/sky_ecs_derive/   QueryData and StageLabel derive macros
crates/sky_type/         Runtime type identity and layout metadata
tools/ecs-comparison/    Canonical fair cross-engine benchmark
```

Sky ECS is also the ECS foundation used by
[SkyEngine](https://github.com/jz315/SkyEngine), which re-exports it as
`sky_engine::ecs`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Query and storage changes must preserve
correctness, drop semantics, and hot-path code quality.

## License

MIT.

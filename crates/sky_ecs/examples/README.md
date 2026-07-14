# Sky ECS Examples

[中文](README_zh.md)

These examples form one progressive learning path. Each program is standalone,
deterministic, and checks its result with assertions.

## Core path

Follow steps 01 through 07 in order for normal game and application code.

| Step | Example | What you learn | Run | Expected result |
|---:|---|---|---|---|
| 01 | [World](step_01_world.rs) | `World`, bundles, direct component access, structural changes, and generational `EntityId` values | `cargo run -p sky_ecs --example step_01_world` | A replacement entity is alive while the stale ID remains invalid. |
| 02 | [Queries](step_02_queries.rs) | Read-only, mutable, optional, named, entity-aware, and filtered typed queries | `cargo run -p sky_ecs --example step_02_queries` | Three moving entities are updated and two active enemies are found. |
| 03 | [Batches and chunks](step_03_batches_and_chunks.rs) | `spawn_batch`, entity iteration, aligned chunk slices, and checksums | `cargo run -p sky_ecs --example step_03_batches_and_chunks` | 10,000 entities are updated and both iteration styles agree. |
| 04 | [Commands](step_04_commands.rs) | Direct structural changes and owned deferred `CommandBuffer` operations | `cargo run -p sky_ecs --example step_04_commands` | One command batch is applied and one cleared batch is discarded. |
| 05 | [Systems](step_05_systems.rs) | Typed system parameters, resources, stages, commands, time, and fixed steps | `cargo run -p sky_ecs --example step_05_systems` | `FixedUpdate`, `Update`, and `PostUpdate` run in stable order. |
| 06 | [Parallel iteration](step_06_parallel.rs) | Equivalent `View` and `ParView` updates with a deterministic checksum | `cargo run -p sky_ecs --example step_06_parallel` | Serial and parallel worlds produce the same checksum. |
| 07 | [Tiny Defense](step_07_tiny_defense.rs) | A complete staged game loop with deferred structural changes | `cargo run -p sky_ecs --example step_07_tiny_defense` | Four ASCII frames end in victory with score 2 and an undamaged base. |

## Advanced path

Steps 08 and 09 are independent advanced topics. Read them after the core path
when your project needs runtime type information or reusable module installation.

| Step | Example | What you learn | Run | Expected result |
|---:|---|---|---|---|
| 08 | [Dynamic API](step_08_dynamic.rs) | Runtime bundles, read/write slots, validation, and error handling for tools and scripting | `cargo run -p sky_ecs --example step_08_dynamic` | A runtime query updates a component and rejects duplicate slots. |
| 09 | [Plugin](step_09_plugin.rs) | Installing configuration and systems through a reusable `Plugin` | `cargo run -p sky_ecs --example step_09_plugin` | Configuration affects movement and duplicate installation is rejected. |

Examples teach API usage; performance measurements live in the repository's
[benchmark guide](../../../benches/BENCHMARKS.md).

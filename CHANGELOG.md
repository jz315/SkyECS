# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- Added `PreparedEntityAccessor<T>` for reusable single-component access by
  arbitrary entity ID without rebuilding its chunk-route table on every bind.

## [0.2.0] - 2026-07-27

- Renamed the bound random-access types to `EntityAccessor` and
  `EntityAccessorMut`; the former `ComponentAccessor` names are removed.
- Added `prepare_access` and `prepare_access_mut` with compact direct-address
  plans for fixed entity sequences, strict validation, and duplicate rejection
  for mutable access.
- Added reusable tuple-capable `PreparedEntityView`, with optional-component
  semantics and route-table reuse across pure row churn.
- Added scheduler-native `EntityView<Q>` for prepared tuple lookup by entity ID
  inside ordinary systems.
- Added a dedicated column-base epoch, explicit route-table shrinking, and
  route-cache diagnostics while preserving live `ChunkId` values.
- Published `sky_type` 0.1.2, allowing distinct Rust `TypeId` values to share a
  registered type name, exposing all ambiguous matches, and keeping opaque
  dynamic names unique.
- Kept prepared-query hot match lists limited to currently active archetype
  storages while retaining incremental signature matching.
- Returned explicit errors for over-wide dynamic bundles and queries, and stored
  long-running elapsed time totals as `f64`.
- Made schedule panic recovery and shutdown attempt every command/state
  cleanup, preserve the first panic, and guarantee teardown at-most-once.
- Restored comparable five-phase gameplay diagnostics and aligned Flecs with
  the per-frame `TargetSlot` data flow used by every other adapter.
- Strengthened Compare-ECS gameplay and random-access contracts with entity
  identity, full component-shape, per-phase checkpoints, and observable
  Velocity, Damage, Regen, Cooldown, and AI-state checks.
- Required release contract validation before publication benchmark runs.
- Separated crate-owned API experiments from the canonical Compare-ECS target,
  split EntityId and fixed-sequence access contracts, and kept fixed-sequence
  access out of the formal GitHub benchmark.
- Kept bulk construction in the Comparable family, starting every adapter from
  neutral component columns and timing complete engine-native batch
  construction and insertion.

## [0.1.3] - 2026-07-18

- Hardened component ownership, destruction, command application, and
  structural transitions, with focused Miri coverage for unsafe storage paths.
- Added adaptive tiered chunk layouts and pooling, including exact one-row
  storage for component sets larger than the standard chunk size.
- Added component-posting query plans and a stable-layout chunk cache to reduce
  repeated archetype and chunk traversal overhead.
- Replaced the original examples with a nine-step progressive learning path and
  expanded the bilingual API and benchmark documentation.
- Reworked Compare-ECS around validated workloads and a statically linked Flecs
  C adapter with explicit compiler and timing boundaries.

## [0.1.2] - 2026-07-14

- Added bound read-only and mutable component accessors for repeated random
  access by entity ID.
- Added accessor correctness, compile-fail, and random-access benchmark
  coverage.
- Expanded the reproducible ECS comparison suite and refreshed documentation.

## [0.1.1] - 2026-07-13

- Moved Sky ECS into its own repository while preserving its source history.
- Made examples, internal benchmarks, and the fair cross-engine comparison
  depend directly on `sky_ecs`.
- Added an independent workspace, CI, release metadata, and bilingual project
  documentation.
- Kept the public ECS API compatible with 0.1.0.

[Unreleased]: https://github.com/jz315/SkyECS/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jz315/SkyECS/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/jz315/SkyECS/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jz315/SkyECS/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jz315/SkyECS/releases/tag/v0.1.1

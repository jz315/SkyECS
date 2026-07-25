# ECS Benchmarks

Compare-ECS measures six ECS libraries through equivalent public APIs. The
formal adapter for each operation uses the fastest supported path certified
for that workload.

## GitHub report

Every completed GitHub report starts with one source link and then presents
exactly four sections in this order:

1. Comparable
2. Random Fragmentation
3. Gameplay Scenario
4. Diagnostic

The engine columns are always `Sky`, `hecs`, `Bevy`, `Flecs C`, `FreeCS`, and
`Shipyard`. Lower is faster. Bold marks the lowest median in a row, `†` marks a
noisy cell, and `N/A` means that no result is available.

## Comparable

The Comparable section has ten rows:

| Test | Scale / Mode |
|---|---|
| Entity construction | Individual 10K |
| Entity construction | Native bulk 10K |
| Entity operations | Spawn/despawn 1K |
| Entity operations | Add/remove component 1K |
| EntityId random access | Hot 10K |
| EntityId random access | Warm 100K |
| Prepared iteration | 10K |
| Prepared iteration | 100K |
| Prepared iteration | 1M |
| Fragmented iteration | 26 × 400 |

Individual and native-bulk construction are two comparable modes in the same
construction family.

## Random Fragmentation

Random Fragmentation is a separate official section. Its name does not include
"ported"; the report acknowledges its external benchmark origin in prose.

The Tag table contains 6 and 8 shapes at 1/4 terms, plus 10 and 16 shapes at
1/4/8 terms. The Data Component table uses the same ten configurations, for a
total of twenty rows.

## Gameplay Scenario

Gameplay is the only scenario family. Its six rows are:

- Full frame
- Iteration
- AI source lookup
- Target Position lookup
- Status transition
- Projectile recycle

Every phase benchmark advances the same complete evolving 65,536-entity,
256-frame state machine as the full frame while timing only the selected
phase.

## Diagnostic

Heavy compute is the single diagnostic row. It is not included in comparative
win counts.

## Local-only experiments

Fixed Sequence Access is not part of the GitHub report. It locally measures
plan build, steady traversal, and build amortized over 1/4/16/64 traversals:

```bash
cargo bench -p sky_ecs_comparison --bench api_candidates \
  --features api-experiments -- fixed_sequence_access
```

API candidates and AB/BA certification are also local-only. GitHub may compile
candidate targets but must not execute them or select a production API from
shared-runner measurements.

## Run

```bash
cargo compare-ecs
cargo compare-ecs -- entity_construction/single_insert_10k/sky --exact
cargo compare-ecs -- random_fragmentation/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- gameplay_scenario/ai_source_lookup/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` runs release contracts before Criterion. A failed
contract stops the run. A complete formal report contains 37 rows: 10
Comparable, 20 Random Fragmentation, 6 Gameplay Scenario, and 1 Diagnostic.
Raw distributions, environment metadata, contracts, and compiler information
remain in the GitHub Actions artifact rather than being copied into the
human-facing report.

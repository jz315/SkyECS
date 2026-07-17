# ECS Benchmarks

Compare-ECS compares ECS libraries through public APIs. Each adapter uses its
fastest suitable reusable query, view, or accessor state. The suite is
single-threaded; it does not make claims about scheduling, parallelism, or
memory use.

Comparable workloads align initial entities and component values, final
archetype/table distribution, query conditions, actual reads and writes, and
timing boundaries. Each ECS may use its fastest official API and its own cache
representation. Prepared state and input data may be created outside timing,
but setup must not pre-create target storage whose creation is part of the
measured operation.

Prepared random access compares reusable lookup state (for example, a Sky
accessor and Flecs refs). Its preparation cost and memory footprint are not
measured. Mixed-frame workloads are scenarios, and heavy compute is a
diagnostic; neither category participates in comparative win counts or an
overall ECS ranking.

The current Flecs adapter links the official C core and comparison adapter
statically and calls them through a direct C ABI. Dynamic-library loading,
symbol lookup, and cached loader checks are therefore outside the measured
path.

## Run

```bash
cargo compare-ecs
cargo compare-ecs -- prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- prepared_iteration/simple_10k/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_components_4_terms/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` runs six Latin-square order rotations and writes raw Criterion data, environment metadata, confidence intervals, and cross-run medians to `target/comparison-reports/`.

## Current results

The table records current-protocol local measurements from 2026-07-17. Most
Rust-adapter cells came from one complete run; after the native compiler audit,
the Flecs column was remeasured with Clang/LLVM 22.1.2. Spawn/random despawn and
random add/remove were remeasured for all adapters after adopting deterministic
shuffled orders. Flecs construction/mixed-spawn cells and Sky's complete/heavy
mixed-frame cells were also remeasured after their targeted audits. Old-protocol
values have been removed. Values are Criterion `median.point_estimate`; bold
marks the lowest median among the adapters that support a workload. This is a
targeted current snapshot, not the six-run publish report.

### General workloads

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **121.723 µs** | 205.140 µs | 307.226 µs | 237.701 µs | 263.850 µs | 282.871 µs |
| Single insert 10k | **190.885 µs** | 485.219 µs | 636.250 µs | 445.943 µs | 916.928 µs | 837.098 µs |
| Prepared iteration 10k | 5.308 µs | 5.354 µs | 7.932 µs | **5.136 µs** | 6.876 µs | 11.483 µs |
| Prepared iteration 100k | 57.936 µs | 58.475 µs | 89.974 µs | **53.715 µs** | 70.766 µs | 119.937 µs |
| Prepared iteration 1m | 1009.044 µs | 1094.400 µs | 1340.761 µs | **880.617 µs** | 1240.763 µs | 2092.052 µs |
| Fragmented iteration 26 × 400 | 0.640 µs | 3.022 µs | 6.037 µs | 2.806 µs | 0.554 µs | **0.417 µs** |
| Diagnostic: heavy compute | **2336.824 µs** | 2413.163 µs | 2421.232 µs | 2883.792 µs | 3014.084 µs | 2664.126 µs |
| Prepared random access 10k | 16.238 µs | 11.263 µs | 30.554 µs | **7.295 µs** | 16.152 µs | 12.570 µs |
| Prepared random access 100k | 433.720 µs | 324.000 µs | 641.115 µs | **147.942 µs** | 403.594 µs | 286.066 µs |
| Spawn/random despawn 1k | 24.756 µs | **22.927 µs** | 79.586 µs | 39.370 µs | 108.852 µs | 64.289 µs |
| Random add/remove component 1k | 55.597 µs | 67.469 µs | 97.514 µs | 78.771 µs | 134.259 µs | **27.461 µs** |
| Mixed frame | 220.182 µs | **218.611 µs** | 254.184 µs | 290.917 µs | 312.643 µs | 287.525 µs |
| Mixed phase: movement | 15.936 µs | **15.069 µs** | 19.819 µs | 16.919 µs | 17.109 µs | 30.737 µs |
| Mixed phase: health × 8 | **5.945 µs** | 17.233 µs | 38.477 µs | 17.875 µs | 27.583 µs | 56.056 µs |
| Mixed phase: heavy | 189.745 µs | 184.635 µs | 190.276 µs | 227.523 µs | **181.982 µs** | 184.432 µs |
| Mixed phase: random access | 1.110 µs | 0.674 µs | 1.753 µs | **0.353 µs** | 0.986 µs | 0.695 µs |
| Mixed phase: structural churn | 10.879 µs | 13.749 µs | 21.125 µs | 17.467 µs | 29.865 µs | **6.442 µs** |
| Mixed phase: spawn/despawn × 32 | **36.556 µs** | 42.186 µs | 122.651 µs | 59.221 µs | 198.404 µs | 155.888 µs |

### Random-fragmentation workloads

The suite follows the complete random-fragmentation matrix from
[Sander Mertens' benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273). FreeCS 3.13.0 registers new tables with work proportional to the existing table count, so its six 16-component cells are `N/A`; their setup does not finish on a practical benchmark timescale.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 tags, 1 term | 2.983 µs | 16.778 µs | 23.681 µs | 3.003 µs | 3.531 µs | **2.583 µs** |
| 6 tags, 4 terms | **0.322 µs** | 2.120 µs | 2.869 µs | 0.343 µs | 0.427 µs | 325.795 µs |
| 8 tags, 1 term | 4.457 µs | 21.151 µs | 23.936 µs | 4.391 µs | 3.887 µs | **3.065 µs** |
| 8 tags, 4 terms | **0.415 µs** | 2.856 µs | 2.908 µs | 0.490 µs | 0.430 µs | 326.298 µs |
| 10 tags, 1 term | 6.694 µs | 35.043 µs | 24.994 µs | 7.334 µs | 5.389 µs | **2.572 µs** |
| 10 tags, 4 terms | 0.710 µs | 5.152 µs | 3.026 µs | 0.816 µs | **0.559 µs** | 332.654 µs |
| 10 tags, 8 terms | **0.031 µs** | 0.428 µs | 0.216 µs | 0.060 µs | 0.042 µs | 385.582 µs |
| 16 tags, 1 term | 390.072 µs | 1154.585 µs | 168.304 µs | 284.854 µs | N/A | **2.757 µs** |
| 16 tags, 4 terms | 26.352 µs | 168.866 µs | **5.594 µs** | 18.859 µs | N/A | 378.894 µs |
| 16 tags, 8 terms | 0.660 µs | 13.821 µs | **0.250 µs** | 0.881 µs | N/A | 424.635 µs |
| 6 data components, 1 term | 38.215 µs | 46.801 µs | 40.150 µs | **23.870 µs** | 43.992 µs | 38.709 µs |
| 6 data components, 4 terms | 17.265 µs | 18.763 µs | 18.532 µs | **10.658 µs** | 19.530 µs | 362.861 µs |
| 8 data components, 1 term | 39.981 µs | 53.121 µs | 43.155 µs | **25.779 µs** | 45.487 µs | 43.931 µs |
| 8 data components, 4 terms | 17.686 µs | 19.704 µs | 18.520 µs | **11.020 µs** | 19.187 µs | 363.577 µs |
| 10 data components, 1 term | 53.989 µs | 77.811 µs | 66.977 µs | **37.752 µs** | 53.129 µs | 38.469 µs |
| 10 data components, 4 terms | 18.500 µs | 22.902 µs | 19.733 µs | **12.302 µs** | 20.191 µs | 368.449 µs |
| 10 data components, 8 terms | 2.584 µs | 2.976 µs | 2.585 µs | **1.609 µs** | 3.050 µs | 424.783 µs |
| 16 data components, 1 term | 962.934 µs | 1567.189 µs | 801.496 µs | 604.828 µs | N/A | **41.431 µs** |
| 16 data components, 4 terms | 135.136 µs | 275.348 µs | 218.724 µs | **103.519 µs** | N/A | 383.880 µs |
| 16 data components, 8 terms | 5.296 µs | 19.148 µs | 6.208 µs | **4.411 µs** | N/A | 450.869 µs |

## Environment

- Windows 11 Pro 10.0.26200, Intel Core i7-12700F (12 cores, 20 logical processors).
- rustc 1.96.0 (`x86_64-pc-windows-msvc`, LLVM 22.1.2).
- Sky ECS 0.1.2, hecs 0.11.0, Bevy ECS 0.19.0, Flecs 4.1.6,
  FreeCS 3.13.0, Shipyard 0.11.5, Criterion 0.8.2.
- Each benchmark uses a 3-second warmup, an at-least-5-second measurement
  target, and 100 samples.
- Rust uses LLVM 22.1.2 with `opt-level = 3`, fat LTO, and one codegen unit.
  The native Flecs archive uses Clang/LLVM `-O3 -flto -DNDEBUG`; `rust-lld`
  performs the final Windows link and native LTO.

## Notes

- Spawn/random despawn prepares one deterministic permutation of the 1,000
  logical entity positions outside timing, then deletes newly spawned entities
  in that order. Random-number generation and shuffling are not measured.
- Random add/remove prepares two independent deterministic permutations outside
  timing: one selects the Health insertion order and the other selects the
  removal order. Each work item is one complete add/remove cycle, so 1,000 work
  items perform 2,000 structural API calls.
- Health and spawn/despawn phases repeat 8 and 32 times; divide by those factors for a single-frame estimate.
- Phase benchmarks use isolated worlds, so they do not sum exactly to the complete mixed frame.

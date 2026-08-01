# ECS Benchmarks

Compare-ECS measures six ECS libraries through equivalent public APIs. Each
formal adapter uses the fastest supported path certified for that workload.

[GitHub Actions run 30705936563](https://github.com/jz315/SkyECS/actions/runs/30705936563)

## Comparable

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

## Random Fragmentation

This section follows Sander Mertens' public random-fragmentation benchmark.

### Tags

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Tags | 1 | 4.024 µs | 38.620 µs† | 30.999 µs | 4.255 µs | **3.663 µs** | 5.117 µs |
| 6 Tags | 4 | 442.065 ns | 3.123 µs† | 3.874 µs | 498.419 ns | **429.859 ns** | 425.192 µs |
| 8 Tags | 1 | 8.177 µs | 45.436 µs† | 33.208 µs | 7.381 µs | 6.835 µs | **5.117 µs** |
| 8 Tags | 4 | 696.163 ns | 3.991 µs† | 3.976 µs | 712.907 ns | **610.077 ns** | 428.397 µs |
| 10 Tags | 1 | 15.628 µs | 58.490 µs | 44.700 µs | 13.049 µs | 11.367 µs† | **5.116 µs** |
| 10 Tags | 4 | 1.179 µs | 7.637 µs | 4.030 µs | 1.222 µs | **827.935 ns** | 428.080 µs |
| 10 Tags | 8 | **47.560 ns** | 431.729 ns | 283.385 ns | 96.230 ns | 60.538 ns | 494.809 µs |
| 16 Tags | 1 | 246.435 µs† | 548.341 µs | 197.198 µs† | 422.379 µs† | N/A | **5.115 µs** |
| 16 Tags | 4 | 29.215 µs† | 171.080 µs | **10.650 µs**† | 56.191 µs† | N/A | 429.848 µs |
| 16 Tags | 8 | 937.198 ns† | 15.260 µs | **410.390 ns**† | 1.667 µs | N/A | 497.604 µs |

### Data Components

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Components | 1 | 44.729 µs | 80.358 µs | 55.728 µs | **42.381 µs** | 51.258 µs | 46.727 µs |
| 6 Components | 4 | **18.252 µs** | 25.602 µs | 22.870 µs | 18.571 µs | 23.832 µs | 450.832 µs |
| 8 Components | 1 | 50.720 µs | 86.264 µs | 62.745 µs | 47.053 µs | 53.967 µs | **46.709 µs** |
| 8 Components | 4 | **18.740 µs** | 29.454 µs | 23.134 µs | 18.809 µs | 23.959 µs | 455.592 µs |
| 10 Components | 1 | 66.018 µs | 104.561 µs | 86.374 µs | 60.824 µs | 63.774 µs | **46.717 µs** |
| 10 Components | 4 | **19.682 µs** | 35.332 µs | 24.287 µs | 19.904 µs | 24.468 µs | 452.694 µs |
| 10 Components | 8 | 3.073 µs | 3.421 µs | 3.051 µs | **2.417 µs** | 3.357 µs | 535.227 µs |
| 16 Components | 1 | 345.745 µs† | 638.925 µs† | 364.205 µs | 543.730 µs† | N/A | **46.714 µs** |
| 16 Components | 4 | **92.601 µs**† | 238.301 µs† | 102.557 µs | 100.671 µs† | N/A | 458.649 µs |
| 16 Components | 8 | 6.781 µs† | 20.750 µs | 6.587 µs | **6.171 µs** | N/A | 542.174 µs |

## Gameplay Scenario

| Gameplay item | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Full frame | **113.920 µs** | 138.687 µs | 201.104 µs | 134.947 µs | 177.867 µs | 311.203 µs |
| Iteration | 54.539 µs | 60.089 µs | 84.272 µs | **50.096 µs** | 87.049 µs | 197.963 µs |
| AI source lookup | 18.407 µs | **17.127 µs** | 37.696 µs | 28.972 µs | 21.524 µs | 29.142 µs |
| Target Position lookup | **15.941 µs** | 16.727 µs | 27.014 µs | 24.871 µs | 17.675 µs | 18.916 µs |
| Status transition | 16.199 µs | 32.444 µs | 30.450 µs | 18.488 µs | 32.629 µs | **7.316 µs** |
| Projectile recycle | **8.201 µs** | 13.178 µs | 22.170 µs | 13.455 µs | 19.116 µs | 54.306 µs |

## Diagnostic

| Diagnostic | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Heavy compute | 3.643 ms | 3.578 ms | 3.671 ms | **3.319 ms** | 3.580 ms | 3.502 ms |

## Notes

Lower is faster. **Bold** marks the lowest median in a row; `†` marks a noisy
shared-runner result; `N/A` means no result. Four rotations do not form a
complete six-engine position block, so order bias is N/A.

The formal report contains 37 rows: 10 Comparable, 20 Random Fragmentation,
6 Gameplay Scenario, and 1 Diagnostic. Gameplay is the only scenario workload;
each phase advances the same evolving 65,536-entity, 256-frame state machine
while timing only the selected phase.

Fixed Sequence Access and API-candidate selection are local experiments and do
not run in the GitHub performance workflow:

```bash
cargo bench -p sky_ecs_comparison --bench api_candidates \
  --features api-experiments -- fixed_sequence_access
```

Run the formal suite with:

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

The publisher runs release contracts before Criterion. Raw distributions,
environment metadata, contracts, and compiler information remain in the linked
GitHub Actions artifact.

Bulk construction starts from neutral component columns and builds each
engine-native batch inside the timed region. This run supersedes the withdrawn
prebuilt-batch result.

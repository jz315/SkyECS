# ECS Benchmarks

Compare-ECS compares Sky ECS with hecs, Bevy ECS, Flecs, FreeCS, and Shipyard through safe public APIs. Each library uses its recommended reusable query, view, or accessor state. The suite is single-threaded; it does not make claims about scheduling, parallelism, or memory use.

## Run

```bash
cargo compare-ecs
cargo compare-ecs -- fair_prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- fair_construction/bulk_insert_10k/flecs --exact
cargo compare-ecs -- fair_diagnostic_flecs_spawn_despawn/direct_1k --exact
cargo compare-ecs -- fair_diagnostic_flecs_spawn_despawn/deferred_1k --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` runs six Latin-square order rotations and writes raw Criterion data, environment metadata, confidence intervals, and cross-run medians to `target/fair-reports/`.

## Recorded results

Cross-run medians from report `1783975835`, recorded on 2026-07-14 on Windows 11, i7-12700F, and Rust 1.96.0. Versions: Sky ECS 0.1.1, hecs 0.11.0, Bevy ECS 0.19.0, flecs_ecs 0.2.2, FreeCS 3.13.0, Shipyard 0.11.5.

Bold marks the lowest median and also marks Sky when it is within 1% of the lowest.

| Workload | Sky | hecs | Bevy | Flecs | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | **146.7 µs** | 242.6 µs | 287.6 µs | 273.84 µs | 261.0 µs | 158.0 µs |
| Single insert 10k | **207.7 µs** | 491.0 µs | 654.2 µs | 3.43 ms | 882.6 µs | 732.7 µs |
| Prepared iteration 10k | **4.96 µs** | 5.10 µs | 7.69 µs | 5.15 µs | 7.78 µs | 11.03 µs |
| Prepared iteration 10k × 32 | **158.364 µs** | 165.264 µs | 241.443 µs | 160.181 µs | 243.145 µs | 315.386 µs |
| Prepared iteration 100k | **52.308 µs** | 55.084 µs | 80.748 µs | **52.013 µs** | 79.492 µs | 114.182 µs |
| Fragmented iteration 26 × 400 | 0.711 µs | 2.507 µs | 5.712 µs | 1.030 µs | 4.774 µs | **0.522 µs** |
| Prepared random access 10k | 15.08 µs | 128.48 µs | 35.10 µs | 314.90 µs | 14.63 µs | **10.16 µs** |
| Prepared random access 100k | 221.952 µs | 1.291 ms | 511.700 µs | 3.214 ms | 224.437 µs | **154.510 µs** |
| Prepared random access 1m | 5.624 ms | 15.090 ms | 16.485 ms | 51.450 ms | 5.737 ms | **3.759 ms** |
| Spawn/despawn 1k | **19.57 µs** | 24.51 µs | 63.54 µs | 157.58 µs | 72.28 µs | 59.71 µs |
| Add/remove component 1k | 45.52 µs | 57.67 µs | 83.60 µs | 118.23 µs | 98.81 µs | **25.17 µs** |
| Diagnostic heavy compute | **1.871 ms** | **1.865 ms** | 1.870 ms | 1.873 ms | 1.871 ms | 1.870 ms |
| Mixed frame | **181.68 µs** | 195.09 µs | 238.81 µs | 223.63 µs | 208.04 µs | 200.05 µs |
| Mixed phase: movement | **12.596 µs** | 13.125 µs | 18.948 µs | **12.550 µs** | 18.983 µs | 26.415 µs |
| Mixed phase: health × 8 | **5.274 µs** | 15.234 µs | 36.988 µs | 6.260 µs | 35.346 µs | 54.606 µs |
| Mixed phase: heavy | **151.747 µs** | 155.301 µs | 153.572 µs | **151.016 µs** | 152.090 µs | 151.769 µs |
| Mixed phase: random access | 2.980 µs | 6.827 µs | 1.889 µs | 16.154 µs | **0.615 µs** | 0.649 µs |
| Mixed phase: structural churn | 10.869 µs | 14.916 µs | 57.986 µs | 30.062 µs | 23.769 µs | **6.382 µs** |
| Mixed phase: spawn/despawn × 32 | **38.367 µs** | 51.285 µs | 541.702 µs | 320.296 µs | 139.507 µs | 147.644 µs |

## Notes

- Health and spawn/despawn phases repeat 8 and 32 times; divide by those factors for a single-frame estimate.
- Phase benchmarks use isolated worlds, so they do not sum exactly to the complete mixed frame.
- Cold 1m random access showed substantial cross-run noise; keep it as data, not a headline claim.

# ECS Benchmarks

Compare-ECS compares ECS libraries through public APIs. Each adapter uses its
fastest suitable reusable query, view, or accessor state. 

## Run

```bash
cargo compare-ecs
cargo compare-ecs -- prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- prepared_iteration/simple_10k/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_components_4_terms/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` defaults to six Latin-square order rotations and writes raw Criterion data, environment metadata, confidence intervals, and cross-run medians to `target/comparison-reports/`. The published run below used four rotations.

## Current results

All values below come from public
[GitHub Actions run #29695552048](https://github.com/jz315/SkyECS/actions/runs/29695552048)

bold marks the lowest median among supported adapters.

### General workloads

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Bulk insert 10k | 120.928 µs | 352.108 µs | 440.190 µs | **110.409 µs** | 278.079 µs | 166.753 µs |
| Single insert 10k | **244.539 µs** | 576.178 µs | 740.224 µs | 741.595 µs | 934.128 µs | 1188.686 µs |
| Prepared iteration 10k | 8.115 µs | 7.834 µs | 9.349 µs | **7.692 µs** | 11.956 µs | 17.292 µs |
| Prepared iteration 100k | 81.214 µs | 78.875 µs | 93.647 µs | **77.616 µs** | 120.081 µs | 174.299 µs |
| Prepared iteration 1m | 885.076 µs | 844.621 µs | 1011.443 µs | **830.301 µs** | 1232.218 µs | 1771.278 µs |
| Fragmented iteration 26 × 400 | 0.852 µs | 4.238 µs | 6.842 µs | 1.104 µs | 0.860 µs | **0.819 µs** |
| Diagnostic: heavy compute | 4118.068 µs | 3569.316 µs | 3664.288 µs | **3325.052 µs** | 3582.510 µs | 3446.964 µs |
| Prepared random access 10k | 21.552 µs | 16.028 µs | 44.423 µs | **9.338 µs** | 23.908 µs | 20.201 µs |
| Prepared random access 100k | 399.066 µs | 294.265 µs | 895.874 µs | **167.122 µs** | 447.005 µs | 450.412 µs |
| Spawn/random despawn 1k | **45.370 µs** | 46.338 µs | 103.052 µs | 67.469 µs | 112.407 µs | 107.419 µs |
| Random add/remove component 1k | 92.425 µs | 111.459 µs | 173.031 µs | 134.730 µs | 212.391 µs | **51.553 µs** |
| Mixed frame | 349.537 µs | 352.553 µs | 377.572 µs | **331.783 µs** | 409.090 µs | 380.534 µs |
| Mixed phase: movement | 20.867 µs | 20.013 µs | 24.135 µs | **19.517 µs** | 30.347 µs | 54.180 µs |
| Mixed phase: health × 8 | 7.904 µs | 22.760 µs | 46.185 µs | **7.746 µs** | 54.003 µs | 98.447 µs |
| Mixed phase: heavy | 300.912 µs | 297.594 µs | 298.251 µs | **268.548 µs** | 305.817 µs | 281.299 µs |
| Mixed phase: random access | 1.133 µs | 0.851 µs | 2.103 µs | **0.498 µs** | 1.204 µs | 1.016 µs |
| Mixed phase: structural churn | 18.731 µs | 24.880 µs | 36.712 µs | 30.973 µs | 52.856 µs | **11.836 µs** |
| Mixed phase: spawn/despawn × 32 | **63.422 µs** | 83.603 µs | 182.251 µs | 125.733 µs | 266.171 µs | 256.373 µs |

### Random-fragmentation workloads

The suite follows the complete random-fragmentation matrix from
[Sander Mertens' benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273). FreeCS 3.13.0 registers new tables with work proportional to the existing table count, so its six 16-component cells are `N/A`; their setup does not finish on a practical benchmark timescale.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 tags, 1 term | 3.976 µs | 39.208 µs | 31.087 µs | 4.304 µs | 5.570 µs | **2.812 µs** |
| 6 tags, 4 terms | **0.449 µs** | 3.157 µs | 3.876 µs | 0.503 µs | 0.708 µs | 412.967 µs |
| 8 tags, 1 term | 7.351 µs | 45.512 µs | 33.575 µs | 7.518 µs | 7.934 µs | **2.782 µs** |
| 8 tags, 4 terms | **0.585 µs** | 4.024 µs | 3.977 µs | 0.711 µs | 0.787 µs | 419.260 µs |
| 10 tags, 1 term | 11.270 µs | 59.638 µs | 46.002 µs | 13.032 µs | 11.769 µs | **2.843 µs** |
| 10 tags, 4 terms | **0.851 µs** | 7.551 µs | 4.035 µs | 1.218 µs | 0.914 µs | 421.389 µs |
| 10 tags, 8 terms | **0.049 µs** | 0.433 µs | 0.283 µs | 0.096 µs | 0.068 µs | 486.700 µs |
| 16 tags, 1 term | 138.696 µs | 539.481 µs | 202.379 µs | 464.624 µs | N/A | **2.784 µs** |
| 16 tags, 4 terms | 11.693 µs | 165.293 µs | **11.136 µs** | 56.867 µs | N/A | 421.217 µs |
| 16 tags, 8 terms | 0.494 µs | 15.565 µs | **0.413 µs** | 1.673 µs | N/A | 494.824 µs |
| 6 data components, 1 term | 44.493 µs | 82.422 µs | 56.062 µs | **42.504 µs** | 51.507 µs | 46.791 µs |
| 6 data components, 4 terms | **18.200 µs** | 25.209 µs | 22.892 µs | 18.585 µs | 23.851 µs | 444.311 µs |
| 8 data components, 1 term | 49.693 µs | 88.360 µs | 63.253 µs | 47.420 µs | 54.726 µs | **46.789 µs** |
| 8 data components, 4 terms | **18.511 µs** | 29.267 µs | 23.165 µs | 18.823 µs | 23.994 µs | 448.004 µs |
| 10 data components, 1 term | 61.558 µs | 106.767 µs | 88.408 µs | 62.021 µs | 65.747 µs | **46.786 µs** |
| 10 data components, 4 terms | **19.094 µs** | 35.656 µs | 24.316 µs | 19.915 µs | 24.561 µs | 449.914 µs |
| 10 data components, 8 terms | 3.076 µs | 3.418 µs | 3.059 µs | **2.419 µs** | 3.368 µs | 549.040 µs |
| 16 data components, 1 term | 193.082 µs | 708.473 µs | 397.584 µs | 735.413 µs | N/A | **46.786 µs** |
| 16 data components, 4 terms | **59.305 µs** | 232.435 µs | 107.351 µs | 105.599 µs | N/A | 452.362 µs |
| 16 data components, 8 terms | **5.386 µs** | 22.120 µs | 6.859 µs | 6.173 µs | N/A | 550.647 µs |

## Environment

- GitHub-hosted `ubuntu-24.04` runner: Ubuntu 24.04.4 LTS, AMD EPYC 7763,
  4 virtual CPUs, 32 MiB shared L3 cache, Microsoft hypervisor.
- rustc 1.97.1 (`x86_64-unknown-linux-gnu`, LLVM 22.1.6).
- Sky ECS 0.1.3, hecs 0.11.0, Bevy ECS 0.19.0, Flecs 4.1.6,
  FreeCS 3.13.0, Shipyard 0.11.5, Criterion 0.8.2.
- Each benchmark uses a 3-second warmup, an at-least-5-second measurement
  target, and 100 samples.
- Rust uses `opt-level = 3`, fat LTO, and one codegen unit. The native Flecs
  archive uses Clang 18.1.3 with `-O3 -flto -DNDEBUG`; Clang and LLD perform
  the final Linux link and native LTO.

##
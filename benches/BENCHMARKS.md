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
cargo bench -p sky_ecs_comparison --bench gameplay_phases -- ai_source_lookup/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` defaults to four cyclic engine-order rotations and writes raw Criterion data, environment metadata, confidence intervals, and cross-run medians to `target/comparison-reports/`. The bounded four-run protocol avoids workflow timeouts but does not cover all six possible engine positions.

## Recorded CI data

The tables below use the controlled four-rotation report from
[GitHub Actions run 29943504762](https://github.com/jz315/SkyECS/actions/runs/29943504762),
commit `3d8910c69c3663d6f1098ae4713e0dd21abf381e`. Release contracts passed and
the report records a clean working tree. GitHub-hosted measurements remain
provisional: four rotations do not provide a complete six-engine order-bias
block, and noisy rows must not support fine-grained claims.

## General workloads

| Workload | Sky | hecs | Bevy | Fle那不对啊cs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| EntityId random access 10k | 16.595 µs | **15.989 µs** | 44.224 µs | 38.338 µs | 23.849 µs | 20.127 µs |
| EntityId random access 100k | 304.899 µs | **299.815 µs** | 870.120 µs | 664.948 µs | 445.007 µs | 441.247 µs |
| Single insert 10k | **273.630 µs** | 575.189 µs | 759.763 µs | 726.486 µs | 871.974 µs | 1213.411 µs |
| Prepared iteration 10k | 7.737 µs | 7.716 µs | 9.462 µs | **7.367 µs** | 11.881 µs | 17.550 µs |
| Prepared iteration 100k | 77.370 µs | 76.890 µs | 94.741 µs | **73.753 µs** | 119.240 µs | 175.582 µs |
| Prepared iteration 1m (noisy) | **809.265 µs** | 839.938 µs | 1031.120 µs | 881.304 µs | 1221.390 µs | 1800.853 µs |
| Fragmented iteration 26 × 400 | 0.994 µs | 4.255 µs | 6.843 µs | 1.172 µs | 0.860 µs | **0.812 µs** |
| Diagnostic: heavy compute | 4050.339 µs | 3583.832 µs | 3663.536 µs | **3353.111 µs** | 3596.537 µs | 3449.776 µs |
| Spawn/despawn 1k | 52.597 µs | **46.080 µs** | 104.186 µs | 68.138 µs | 110.889 µs | 112.367 µs |
| Add/remove component 1k | 107.409 µs | 108.837 µs | 163.619 µs | 132.399 µs | 229.765 µs | **51.432 µs** |
| Scenario: canonical gameplay frame | **121.680 µs** | 150.293 µs | 211.163 µs | 139.380 µs | 183.381 µs | 330.224 µs |

## Fixed-sequence access scenarios

These are intentionally separate from EntityId random access: they include a
reusable fixed sequence, explicit plan construction, or amortized construction
across the stated number of traversals. The 10k and 100k plan payloads are
80,000 B and 800,000 B respectively.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Build plan 10k | 22.093 µs | **16.249 µs** | 40.453 µs | 43.156 µs | 24.588 µs | 17.192 µs |
| Build plan 100k | 325.150 µs | **238.507 µs** | 620.690 µs | 585.822 µs | 384.074 µs | 277.085 µs |
| Steady traversal 10k | 5.868 µs | **5.841 µs** | 5.847 µs | 5.870 µs | 5.849 µs | 5.847 µs |
| Steady traversal 100k | 83.631 µs | 89.174 µs | 83.567 µs | 76.938 µs | **76.691 µs** | 76.984 µs |
| Build + 1 traversal, 10k | 28.181 µs | **22.160 µs** | 48.093 µs | 50.222 µs | 30.653 µs | 23.931 µs |
| Build + 4 traversals, 10k | 45.683 µs | **39.645 µs** | 65.163 µs | 68.664 µs | 48.160 µs | 41.417 µs |
| Build + 16 traversals, 10k | 115.575 µs | **109.552 µs** | 135.442 µs | 141.727 µs | 118.142 µs | 111.374 µs |
| Build + 64 traversals, 10k | 395.085 µs | **388.708 µs** | 416.136 µs | 433.583 µs | 397.804 µs | 390.847 µs |
| Build + 1 traversal, 100k | 412.480 µs | **331.373 µs** | 704.242 µs | 668.349 µs | 465.369 µs | 359.073 µs |
| Build + 4 traversals, 100k | 661.797 µs | **576.504 µs** | 926.677 µs | 939.068 µs | 696.923 µs | 643.129 µs |
| Build + 16 traversals, 100k | 1561.714 µs | 1576.206 µs | 1858.329 µs | 1989.409 µs | 1827.861 µs | **1517.669 µs** |
| Build + 64 traversals, 100k | 5292.410 µs | 5570.296 µs | 5555.433 µs | 6451.518 µs | 6136.340 µs | **5215.339 µs** |

## Native capability scenario

Native bulk construction uses each engine's specialized bulk API and is a
scenario rather than a comparable public-API workload.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Native bulk construction 10k | 82.397 µs | **13.615 µs** | 462.218 µs | 118.960 µs | 332.428 µs | 197.624 µs |

## Random-fragmentation workloads

The suite follows the complete random-fragmentation matrix from
[Sander Mertens' benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273). FreeCS 3.13.0 registers new tables with work proportional to the existing table count, so its six 16-component cells are `N/A`; their setup does not finish on a practical benchmark timescale.

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 tags, 1 term | 3.908 µs | 38.738 µs | 31.062 µs | 5.838 µs | **3.787 µs** | 5.122 µs |
| 6 tags, 4 terms | 0.443 µs | 3.329 µs | 3.877 µs | 0.749 µs | **0.433 µs** | 426.339 µs |
| 8 tags, 1 term | 8.266 µs | 45.125 µs | 33.732 µs | 8.388 µs | 6.935 µs | **5.122 µs** |
| 8 tags, 4 terms | 0.840 µs | 4.201 µs | 3.977 µs | 0.883 µs | **0.611 µs** | 429.160 µs |
| 10 tags, 1 term | 15.742 µs | 58.761 µs | 45.439 µs | 14.348 µs | 11.696 µs | **5.122 µs** |
| 10 tags, 4 terms | 1.191 µs | 7.461 µs | 4.036 µs | 1.260 µs | **0.831 µs** | 433.047 µs |
| 10 tags, 8 terms | **0.049 µs** | 0.432 µs | 0.284 µs | 0.112 µs | 0.060 µs | 495.359 µs |
| 16 tags, 1 term | 247.513 µs | 535.284 µs | 200.162 µs | 411.737 µs | N/A | **5.124 µs** |
| 16 tags, 4 terms | 29.398 µs | 163.532 µs | **11.163 µs** | 52.098 µs | N/A | 431.424 µs |
| 16 tags, 8 terms | 0.949 µs | 16.119 µs | **0.399 µs** | 1.677 µs | N/A | 502.886 µs |
| 6 data components, 1 term | 44.844 µs | 81.238 µs | 56.093 µs | **36.169 µs** | 51.344 µs | 46.773 µs |
| 6 data components, 4 terms | **18.257 µs** | 26.483 µs | 22.892 µs | 18.619 µs | 23.846 µs | 452.504 µs |
| 8 data components, 1 term | 50.987 µs | 87.151 µs | 63.304 µs | **41.017 µs** | 54.286 µs | 46.755 µs |
| 8 data components, 4 terms | **18.744 µs** | 29.307 µs | 23.153 µs | 18.849 µs | 23.985 µs | 456.392 µs |
| 10 data components, 1 term | 65.548 µs | 105.857 µs | 86.869 µs | 56.164 µs | 64.486 µs | **46.755 µs** |
| 10 data components, 4 terms | **19.818 µs** | 35.558 µs | 24.271 µs | 19.929 µs | 24.550 µs | 458.092 µs |
| 10 data components, 8 terms | 3.073 µs | 3.417 µs | 3.059 µs | **2.420 µs** | 3.363 µs | 539.941 µs |
| 16 data components, 1 term | 337.524 µs | 637.081 µs | 368.138 µs | 547.788 µs | N/A | **46.746 µs** |
| 16 data components, 4 terms | **91.897 µs** | 228.418 µs | 104.667 µs | 104.165 µs | N/A | 459.952 µs |
| 16 data components, 8 terms | 6.682 µs | 21.828 µs | 6.743 µs | **6.196 µs** | N/A | 544.406 µs |

## Environment

- GitHub-hosted `ubuntu-24.04` runner: Ubuntu 24.04.4 LTS, AMD EPYC 7763,
  4 virtual CPUs, 32 MiB shared L3 cache, Microsoft hypervisor.
- rustc 1.97.1 (`x86_64-unknown-linux-gnu`, LLVM 22.1.6).
- Sky ECS 0.2.0, hecs 0.11.0, Bevy ECS 0.19.0, Flecs 4.1.6,
  FreeCS 3.13.0, Shipyard 0.11.5, Criterion 0.8.2.
- Each benchmark uses a 3-second warmup, an at-least-5-second measurement
  target, and 100 samples.
- Rust uses `opt-level = 3`, fat LTO, and one codegen unit. The native Flecs
  archive uses Clang 18.1.3 with `-O3 -flto -DNDEBUG`; Clang and LLD perform
  the final Linux link and native LTO.

## Notes

- GitHub-hosted runners provide public provenance but not dedicated-machine
  isolation. Reports include the runner, toolchain, compiler, commit, raw
  Criterion distributions, and all four recorded order rotations so claims can be
  independently inspected.
- The retired Mixed frame was rejected because eight matrix inversions consumed
  more than 80% of several adapters' frame time, while health and spawn phases
  used artificial repetition factors. It is not a canonical result.

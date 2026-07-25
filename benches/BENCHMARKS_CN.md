# ECS Benchmark

Compare-ECS 通过等效 API 对比多个 ECS 库。各 adapter 都使用适合该
workload 的最快可复用 query、view 或 accessor。

## 运行

```bash
cargo compare-ecs
cargo compare-ecs -- prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- prepared_iteration/simple_10k/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_components_4_terms/sky --exact
cargo bench -p sky_ecs_comparison --bench gameplay_phases -- ai_source_lookup/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` 默认执行四轮循环引擎顺序轮换，并将 Criterion 原始数据、环境信息、置信区间和跨运行中位数写入 `target/comparison-reports/`。四轮有界协议用于避免 workflow 超时，但不会覆盖六个引擎的全部顺序位置。

## 已记录的 CI 数据

下表使用受控的四轮报告：[GitHub Actions run 29943504762](https://github.com/jz315/SkyECS/actions/runs/29943504762)，
提交为 `3d8910c69c3663d6f1098ae4713e0dd21abf381e`。release contracts 已通过，
报告记录的工作树为干净状态。GitHub-hosted runner 的测量仍属 provisional：四轮不足以形成
完整的六引擎顺序偏差区块，波动行不能用于细粒度性能结论。




## 通用 workloads

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| EntityId 随机访问 1 万 | 16.595 µs | **15.989 µs** | 44.224 µs | 38.338 µs | 23.849 µs | 20.127 µs |
| EntityId 随机访问 10 万 | 304.899 µs | **299.815 µs** | 870.120 µs | 664.948 µs | 445.007 µs | 441.247 µs |
| 单个插入 1 万 | **273.630 µs** | 575.189 µs | 759.763 µs | 726.486 µs | 871.974 µs | 1213.411 µs |
| Prepared 遍历 1 万 | 7.737 µs | 7.716 µs | 9.462 µs | **7.367 µs** | 11.881 µs | 17.550 µs |
| Prepared 遍历 10 万 | 77.370 µs | 76.890 µs | 94.741 µs | **73.753 µs** | 119.240 µs | 175.582 µs |
| Prepared 遍历 100 万（波动） | **809.265 µs** | 839.938 µs | 1031.120 µs | 881.304 µs | 1221.390 µs | 1800.853 µs |
| 碎片遍历 26 × 400 | 0.994 µs | 4.255 µs | 6.843 µs | 1.172 µs | 0.860 µs | **0.812 µs** |
| Diagnostic: heavy compute | 4050.339 µs | 3583.832 µs | 3663.536 µs | **3353.111 µs** | 3596.537 µs | 3449.776 µs |
| Spawn/despawn 1 千 | 52.597 µs | **46.080 µs** | 104.186 µs | 68.138 µs | 110.889 µs | 112.367 µs |
| Add/remove component 1 千 | 107.409 µs | 108.837 µs | 163.619 µs | 132.399 µs | 229.765 µs | **51.432 µs** |
| Scenario：canonical gameplay frame | **121.680 µs** | 150.293 µs | 211.163 µs | 139.380 µs | 183.381 µs | 330.224 µs |

## 固定序列访问场景

这类 workload 与 EntityId 随机访问有意分开：它们使用可复用固定序列、显式 plan
构建，或把构建成本摊销到所示次数的 traversal 中。10k 和 100k 的 plan payload
分别为 80,000 B 与 800,000 B。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 构建 plan 1 万 | 22.093 µs | **16.249 µs** | 40.453 µs | 43.156 µs | 24.588 µs | 17.192 µs |
| 构建 plan 10 万 | 325.150 µs | **238.507 µs** | 620.690 µs | 585.822 µs | 384.074 µs | 277.085 µs |
| 稳定 traversal 1 万 | 5.868 µs | **5.841 µs** | 5.847 µs | 5.870 µs | 5.849 µs | 5.847 µs |
| 稳定 traversal 10 万 | 83.631 µs | 89.174 µs | 83.567 µs | 76.938 µs | **76.691 µs** | 76.984 µs |
| 构建 + 1 次 traversal，1 万 | 28.181 µs | **22.160 µs** | 48.093 µs | 50.222 µs | 30.653 µs | 23.931 µs |
| 构建 + 4 次 traversal，1 万 | 45.683 µs | **39.645 µs** | 65.163 µs | 68.664 µs | 48.160 µs | 41.417 µs |
| 构建 + 16 次 traversal，1 万 | 115.575 µs | **109.552 µs** | 135.442 µs | 141.727 µs | 118.142 µs | 111.374 µs |
| 构建 + 64 次 traversal，1 万 | 395.085 µs | **388.708 µs** | 416.136 µs | 433.583 µs | 397.804 µs | 390.847 µs |
| 构建 + 1 次 traversal，10 万 | 412.480 µs | **331.373 µs** | 704.242 µs | 668.349 µs | 465.369 µs | 359.073 µs |
| 构建 + 4 次 traversal，10 万 | 661.797 µs | **576.504 µs** | 926.677 µs | 939.068 µs | 696.923 µs | 643.129 µs |
| 构建 + 16 次 traversal，10 万 | 1561.714 µs | 1576.206 µs | 1858.329 µs | 1989.409 µs | 1827.861 µs | **1517.669 µs** |
| 构建 + 64 次 traversal，10 万 | 5292.410 µs | 5570.296 µs | 5555.433 µs | 6451.518 µs | 6136.340 µs | **5215.339 µs** |

## 原生能力场景

原生 bulk construction 使用各引擎专用的批量 API，属于 scenario，而不是可比较的
公开 API workload。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 原生 bulk construction 1 万 | 82.397 µs | **13.615 µs** | 462.218 µs | 118.960 µs | 332.428 µs | 197.624 µs |

## 随机碎片 workloads

测试矩阵来自 [Sander Mertens benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273)。FreeCS 3.13.0 注册新 table 的成本会随已有 table 数量增长，因此它的六个 16 组件单元格记为 `N/A`；其准备阶段无法在实用的 benchmark 时间内完成。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 Tags，1 term | 3.908 µs | 38.738 µs | 31.062 µs | 5.838 µs | **3.787 µs** | 5.122 µs |
| 6 Tags，4 terms | 0.443 µs | 3.329 µs | 3.877 µs | 0.749 µs | **0.433 µs** | 426.339 µs |
| 8 Tags，1 term | 8.266 µs | 45.125 µs | 33.732 µs | 8.388 µs | 6.935 µs | **5.122 µs** |
| 8 Tags，4 terms | 0.840 µs | 4.201 µs | 3.977 µs | 0.883 µs | **0.611 µs** | 429.160 µs |
| 10 Tags，1 term | 15.742 µs | 58.761 µs | 45.439 µs | 14.348 µs | 11.696 µs | **5.122 µs** |
| 10 Tags，4 terms | 1.191 µs | 7.461 µs | 4.036 µs | 1.260 µs | **0.831 µs** | 433.047 µs |
| 10 Tags，8 terms | **0.049 µs** | 0.432 µs | 0.284 µs | 0.112 µs | 0.060 µs | 495.359 µs |
| 16 Tags，1 term | 247.513 µs | 535.284 µs | 200.162 µs | 411.737 µs | N/A | **5.124 µs** |
| 16 Tags，4 terms | 29.398 µs | 163.532 µs | **11.163 µs** | 52.098 µs | N/A | 431.424 µs |
| 16 Tags，8 terms | 0.949 µs | 16.119 µs | **0.399 µs** | 1.677 µs | N/A | 502.886 µs |
| 6 数据组件，1 term | 44.844 µs | 81.238 µs | 56.093 µs | **36.169 µs** | 51.344 µs | 46.773 µs |
| 6 数据组件，4 terms | **18.257 µs** | 26.483 µs | 22.892 µs | 18.619 µs | 23.846 µs | 452.504 µs |
| 8 数据组件，1 term | 50.987 µs | 87.151 µs | 63.304 µs | **41.017 µs** | 54.286 µs | 46.755 µs |
| 8 数据组件，4 terms | **18.744 µs** | 29.307 µs | 23.153 µs | 18.849 µs | 23.985 µs | 456.392 µs |
| 10 数据组件，1 term | 65.548 µs | 105.857 µs | 86.869 µs | 56.164 µs | 64.486 µs | **46.755 µs** |
| 10 数据组件，4 terms | **19.818 µs** | 35.558 µs | 24.271 µs | 19.929 µs | 24.550 µs | 458.092 µs |
| 10 数据组件，8 terms | 3.073 µs | 3.417 µs | 3.059 µs | **2.420 µs** | 3.363 µs | 539.941 µs |
| 16 数据组件，1 term | 337.524 µs | 637.081 µs | 368.138 µs | 547.788 µs | N/A | **46.746 µs** |
| 16 数据组件，4 terms | **91.897 µs** | 228.418 µs | 104.667 µs | 104.165 µs | N/A | 459.952 µs |
| 16 数据组件，8 terms | 6.682 µs | 21.828 µs | 6.743 µs | **6.196 µs** | N/A | 544.406 µs |

## 测试环境

- GitHub-hosted `ubuntu-24.04` runner：Ubuntu 24.04.4 LTS、AMD EPYC 7763、
  4 个虚拟 CPU、32 MiB 共享 L3 缓存、Microsoft hypervisor。
- rustc 1.97.1（`x86_64-unknown-linux-gnu`，LLVM 22.1.6）。
- Sky ECS 0.2.0、hecs 0.11.0、Bevy ECS 0.19.0、Flecs 4.1.6、
  FreeCS 3.13.0、Shipyard 0.11.5、Criterion 0.8.2。
- 每项预热 3 秒，目标测量时间至少 5 秒，共 100 个样本。
- Rust 使用 `opt-level = 3`、fat LTO 和单 codegen unit；原生 Flecs
  静态库使用 Clang 18.1.3 与 `-O3 -flto -DNDEBUG`，Linux 最终链接和
  原生 LTO 由 Clang 与 LLD 完成。

## 说明

- GitHub-hosted runner 提供公开可追溯性，但不等于专用机器的隔离环境。
  报告会保存 runner、toolchain、编译器、commit、Criterion 原始分布与
  四轮记录完备的顺序轮换，便于独立审查。
- 旧 Mixed frame 被否决的原因是：八次矩阵求逆在多个 adapter 中占帧
  时间 80% 以上，而 health 与 spawn phase 又使用人为重复系数。它不再是
  canonical 结果。

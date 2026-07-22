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




## 通用 workloads

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 旧行式 batch 插入 1 万（已退役） | 120.928 µs | 352.108 µs | 440.190 µs | **110.409 µs** | 278.079 µs | 166.753 µs |
| 单个插入 1 万 | **244.539 µs** | 576.178 µs | 740.224 µs | 741.595 µs | 934.128 µs | 1188.686 µs |
| Prepared 遍历 1 万 | 8.115 µs | 7.834 µs | 9.349 µs | **7.692 µs** | 11.956 µs | 17.292 µs |
| Prepared 遍历 10 万 | 81.214 µs | 78.875 µs | 93.647 µs | **77.616 µs** | 120.081 µs | 174.299 µs |
| Prepared 遍历 100 万 | 885.076 µs | 844.621 µs | 1011.443 µs | **830.301 µs** | 1232.218 µs | 1771.278 µs |
| 碎片遍历 26 × 400 | 0.852 µs | 4.238 µs | 6.842 µs | 1.104 µs | 0.860 µs | **0.819 µs** |
| heavy compute | 4118.068 µs | 3569.316 µs | 3664.288 µs | **3325.052 µs** | 3582.510 µs | 3446.964 µs |
| 旧非对称随机访问 1 万（已退役） | 21.552 µs | 16.028 µs | 44.423 µs | **9.338 µs** | 23.908 µs | 20.201 µs |
| 旧非对称随机访问 10 万（已退役） | 399.066 µs | 294.265 µs | 895.874 µs | **167.122 µs** | 447.005 µs | 450.412 µs |
| Spawn/随机 despawn 1 千 | **45.370 µs** | 46.338 µs | 103.052 µs | 67.469 µs | 112.407 µs | 107.419 µs |
| 随机 add/remove component 1 千 | 92.425 µs | 111.459 µs | 173.031 µs | 134.730 µs | 212.391 µs | **51.553 µs** |

## 随机碎片 workloads

测试矩阵来自 [Sander Mertens benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273)。FreeCS 3.13.0 注册新 table 的成本会随已有 table 数量增长，因此它的六个 16 组件单元格记为 `N/A`；其准备阶段无法在实用的 benchmark 时间内完成。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 Tags，1 term | 3.976 µs | 39.208 µs | 31.087 µs | 4.304 µs | 5.570 µs | **2.812 µs** |
| 6 Tags，4 terms | **0.449 µs** | 3.157 µs | 3.876 µs | 0.503 µs | 0.708 µs | 412.967 µs |
| 8 Tags，1 term | 7.351 µs | 45.512 µs | 33.575 µs | 7.518 µs | 7.934 µs | **2.782 µs** |
| 8 Tags，4 terms | **0.585 µs** | 4.024 µs | 3.977 µs | 0.711 µs | 0.787 µs | 419.260 µs |
| 10 Tags，1 term | 11.270 µs | 59.638 µs | 46.002 µs | 13.032 µs | 11.769 µs | **2.843 µs** |
| 10 Tags，4 terms | **0.851 µs** | 7.551 µs | 4.035 µs | 1.218 µs | 0.914 µs | 421.389 µs |
| 10 Tags，8 terms | **0.049 µs** | 0.433 µs | 0.283 µs | 0.096 µs | 0.068 µs | 486.700 µs |
| 16 Tags，1 term | 138.696 µs | 539.481 µs | 202.379 µs | 464.624 µs | N/A | **2.784 µs** |
| 16 Tags，4 terms | 11.693 µs | 165.293 µs | **11.136 µs** | 56.867 µs | N/A | 421.217 µs |
| 16 Tags，8 terms | 0.494 µs | 15.565 µs | **0.413 µs** | 1.673 µs | N/A | 494.824 µs |
| 6 数据组件，1 term | 44.493 µs | 82.422 µs | 56.062 µs | **42.504 µs** | 51.507 µs | 46.791 µs |
| 6 数据组件，4 terms | **18.200 µs** | 25.209 µs | 22.892 µs | 18.585 µs | 23.851 µs | 444.311 µs |
| 8 数据组件，1 term | 49.693 µs | 88.360 µs | 63.253 µs | 47.420 µs | 54.726 µs | **46.789 µs** |
| 8 数据组件，4 terms | **18.511 µs** | 29.267 µs | 23.165 µs | 18.823 µs | 23.994 µs | 448.004 µs |
| 10 数据组件，1 term | 61.558 µs | 106.767 µs | 88.408 µs | 62.021 µs | 65.747 µs | **46.786 µs** |
| 10 数据组件，4 terms | **19.094 µs** | 35.656 µs | 24.316 µs | 19.915 µs | 24.561 µs | 449.914 µs |
| 10 数据组件，8 terms | 3.076 µs | 3.418 µs | 3.059 µs | **2.419 µs** | 3.368 µs | 549.040 µs |
| 16 数据组件，1 term | 193.082 µs | 708.473 µs | 397.584 µs | 735.413 µs | N/A | **46.786 µs** |
| 16 数据组件，4 terms | **59.305 µs** | 232.435 µs | 107.351 µs | 105.599 µs | N/A | 452.362 µs |
| 16 数据组件，8 terms | **5.386 µs** | 22.120 µs | 6.859 µs | 6.173 µs | N/A | 550.647 µs |

## 测试环境

- GitHub-hosted `ubuntu-24.04` runner：Ubuntu 24.04.4 LTS、AMD EPYC 7763、
  4 个虚拟 CPU、32 MiB 共享 L3 缓存、Microsoft hypervisor。
- rustc 1.97.1（`x86_64-unknown-linux-gnu`，LLVM 22.1.6）。
- Sky ECS 0.1.3、hecs 0.11.0、Bevy ECS 0.19.0、Flecs 4.1.6、
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

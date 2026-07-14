# ECS Benchmark

Compare-ECS 使用公共 API 对比七个 ECS 实现。各实现都使用适合该 workload 的最快可复用 query、view 或 accessor。当前只测单线程，不代表调度、并行或内存表现。

## 运行

```bash
cargo compare-ecs
cargo compare-ecs -- fair_prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- fair_construction/bulk_insert_10k/flecs --exact
cargo compare-ecs -- fair_prepared_iteration/simple_10k/flecs_cpp --exact
cargo compare-ecs -- fair_diagnostic_flecs_spawn_despawn/direct_1k --exact
cargo compare-ecs -- fair_diagnostic_flecs_spawn_despawn/deferred_1k --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` 执行七轮 Latin-square 顺序轮换，并将 Criterion 原始数据、环境信息、置信区间和跨运行中位数写入 `target/fair-reports/`。

## 实测结果

数据来自六轮报告 `1783975835`，记录于 2026-07-14：Windows 11、i7-12700F、Rust 1.96.0。版本：Sky ECS 0.1.1、hecs 0.11.0、Bevy ECS 0.19.0、flecs_ecs 0.2.2、FreeCS 3.13.0、Shipyard 0.11.5。

加粗表示该项最低中位数，以及与最低值相差不超过 1% 的 Sky 结果。

| Workload | Sky | hecs | Bevy | Flecs | Flecs C++ | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|---:|
| 批量插入 1 万 | **146.7 µs** | 242.6 µs | 287.6 µs | 273.84 µs | 223.310 µs | 261.0 µs | 158.0 µs |
| 单个插入 1 万 | **207.7 µs** | 491.0 µs | 654.2 µs | 3.43 ms | 2.952 ms | 882.6 µs | 732.7 µs |
| Prepared 遍历 1 万 | **4.96 µs** | 5.10 µs | 7.69 µs | 5.15 µs | 8.094 µs | 7.78 µs | 11.03 µs |
| Prepared 遍历 1 万 × 32 | **158.364 µs** | 165.264 µs | 241.443 µs | 160.181 µs | 243.220 µs | 243.145 µs | 315.386 µs |
| Prepared 遍历 10 万 | **52.308 µs** | 55.084 µs | 80.748 µs | **52.013 µs** | 63.463 µs | 79.492 µs | 114.182 µs |
| 碎片遍历 26 × 400 | 0.711 µs | 2.507 µs | 5.712 µs | 1.030 µs | 3.218 µs | 4.774 µs | **0.522 µs** |
| Prepared 随机访问 1 万 | 15.08 µs | 128.48 µs | 35.10 µs | 314.90 µs | 141.880 µs | 14.63 µs | **10.16 µs** |
| Prepared 随机访问 10 万 | 221.952 µs | 1.291 ms | 511.700 µs | 3.214 ms | 2.152 ms | 224.437 µs | **154.510 µs** |
| Prepared 随机访问 100 万 | 5.624 ms | 15.090 ms | 16.485 ms | 51.450 ms | 75.622 ms | 5.737 ms | **3.759 ms** |
| Spawn/despawn 1 千 | **19.57 µs** | 24.51 µs | 63.54 µs | 157.58 µs | 145.020 µs | 72.28 µs | 59.71 µs |
| Add/remove component 1 千 | 45.52 µs | 57.67 µs | 83.60 µs | 118.23 µs | 99.694 µs | 98.81 µs | **25.17 µs** |
| 诊断项：heavy compute | **1.871 ms** | **1.865 ms** | 1.870 ms | 1.873 ms | 2.115 ms | 1.871 ms | 1.870 ms |
| 混合帧 | **181.68 µs** | 195.09 µs | 238.81 µs | 223.63 µs | 222.830 µs | 208.04 µs | 200.05 µs |
| 混合帧阶段：movement | **12.596 µs** | 13.125 µs | 18.948 µs | **12.550 µs** | 16.098 µs | 18.983 µs | 26.415 µs |
| 混合帧阶段：health × 8 | **5.274 µs** | 15.234 µs | 36.988 µs | 6.260 µs | 16.473 µs | 35.346 µs | 54.606 µs |
| 混合帧阶段：heavy | **151.747 µs** | 155.301 µs | 153.572 µs | **151.016 µs** | 156.990 µs | 152.090 µs | 151.769 µs |
| 混合帧阶段：随机访问 | 2.980 µs | 6.827 µs | 1.889 µs | 16.154 µs | 6.755 µs | **0.615 µs** | 0.649 µs |
| 混合帧阶段：结构变更 | 10.869 µs | 14.916 µs | 57.986 µs | 30.062 µs | 22.733 µs | 23.769 µs | **6.382 µs** |
| 混合帧阶段：spawn/despawn × 32 | **38.367 µs** | 51.285 µs | 541.702 µs | 320.296 µs | 270.430 µs | 139.507 µs | 147.644 µs |

## 说明

- Health 和 spawn/despawn 分别重复 8 次和 32 次；换算单帧时需除以对应次数。
- 阶段测试使用独立 World，不会与完整混合帧完全相加。
- 冷随机访问 100 万实体的跨轮噪声较大，只保留数据，不作为首页结论。

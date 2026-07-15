# ECS Benchmark

Compare-ECS 使用公共 API 对比七个 ECS 实现。各实现都使用适合该 workload 的最快可复用 query、view 或 accessor。当前只测单线程，不代表调度、并行或内存表现。

## 运行

```bash
cargo compare-ecs
cargo compare-ecs -- fair_prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- fair_prepared_construction/bulk_insert_10k/flecs --exact
cargo compare-ecs -- fair_prepared_iteration/simple_10k/flecs_cpp --exact
cargo compare-ecs -- fair_prepared_random_fragmented_iteration/random_16_components_4_terms/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` 执行七轮 Latin-square 顺序轮换，并将 Criterion 原始数据、环境信息、置信区间和跨运行中位数写入 `target/fair-reports/`。

## 实测结果

数据记录于 2026-07-15 至 2026-07-16：Windows 11、i7-12700F、Rust 1.96.0。版本：Sky ECS 0.1.2、hecs 0.11.0、Bevy ECS 0.19.0、flecs_ecs 0.2.2 / flecs_ecs_sys 0.2.1（Flecs 4.1.2）、原生 Flecs 4.1.6、FreeCS 3.13.0、Shipyard 0.11.5。

每轮预热 3 秒，目标测量时间至少 5 秒，共 100 个样本。大部分单元格来自七轮报告 `1784070754`；构造测试行以及受 adapter 审计影响的 Flecs 单元格，已替换为修正协议下的单轮定向测量。因此当前表格是审计期间的临时快照，不是一份统一的七轮正式报告。Rust benchmark 使用 opt-level 3、fat LTO 和单 codegen unit；原生代码在 MSVC 下使用 `/O2 /GL /LTCG /DNDEBUG`。

加粗只表示该行展示数据中的最低中位数。

### 通用 workloads

| Workload | Sky | hecs | Bevy | Flecs | Flecs C++ | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|---:|
| 批量插入 1 万 | **145.190 µs** | 202.710 µs | 292.650 µs | 236.920 µs | 208.840 µs | 265.580 µs | 203.980 µs |
| 单个插入 1 万 | **203.260 µs** | 462.420 µs | 620.530 µs | 1.463 ms | 816.190 µs | 882.020 µs | 850.420 µs |
| Prepared 遍历 1 万 | **5.210 µs** | 5.369 µs | 8.081 µs | 5.462 µs | 6.329 µs | 6.877 µs | 11.331 µs |
| Prepared 遍历 1 万 × 32 | **169.053 µs** | 186.137 µs | 256.030 µs | 178.321 µs | 201.566 µs | 221.393 µs | 363.175 µs |
| Prepared 遍历 10 万 | **55.760 µs** | 56.823 µs | 85.049 µs | 57.751 µs | 68.062 µs | 70.409 µs | 116.538 µs |
| 碎片遍历 26 × 400 | 0.759 µs | 2.831 µs | 5.977 µs | 0.864 µs | 2.638 µs | 3.514 µs | **0.425 µs** |
| 诊断项：heavy compute | 2.686 ms | 2.704 ms | 2.701 ms | 2.668 ms | **2.112 ms** | 2.697 ms | 2.698 ms |
| Prepared 随机访问 1 万 | 15.417 µs | **11.335 µs** | 31.256 µs | 19.897 µs | 16.855 µs | 16.108 µs | 11.629 µs |
| Prepared 随机访问 10 万 | 363.353 µs | 271.961 µs | 622.662 µs | 552.997 µs | 476.131 µs | 369.625 µs | **266.522 µs** |
| Prepared 随机访问 100 万 | 19.286 ms | **14.444 ms** | 34.059 ms | 22.042 ms | 18.804 ms | 22.808 ms | 14.522 ms |
| Spawn/despawn 1 千 | **16.838 µs** | 20.555 µs | 61.772 µs | 38.421 µs | 23.377 µs | 93.516 µs | 62.399 µs |
| Add/remove component 1 千 | 42.137 µs | 55.100 µs | 82.276 µs | 123.161 µs | 86.407 µs | 93.187 µs | **25.706 µs** |
| 混合帧 | **183.609 µs** | 260.778 µs | 273.461 µs | 207.863 µs | 219.231 µs | 274.591 µs | 207.077 µs |
| 混合帧阶段：movement | **13.398 µs** | 13.662 µs | 19.909 µs | 13.439 µs | 17.305 µs | 17.331 µs | 30.372 µs |
| 混合帧阶段：health × 8 | **5.515 µs** | 14.384 µs | 38.776 µs | 6.186 µs | 18.232 µs | 28.821 µs | 56.449 µs |
| 混合帧阶段：heavy | 153.917 µs | 219.528 µs | 218.822 µs | 153.630 µs | 172.184 µs | 217.562 µs | **153.279 µs** |
| 混合帧阶段：随机访问 | 0.948 µs | 0.671 µs | 1.760 µs | 0.570 µs | **0.509 µs** | 0.978 µs | 0.699 µs |
| 混合帧阶段：结构变更 | 9.999 µs | 13.979 µs | 20.060 µs | 31.313 µs | 21.315 µs | 22.686 µs | **6.521 µs** |
| 混合帧阶段：spawn/despawn × 32 | **33.105 µs** | 41.747 µs | 105.745 µs | 75.959 µs | 54.802 µs | 184.668 µs | 154.394 µs |

### 随机碎片 benchmark

| 组件数 | Sky | hecs | Bevy | Flecs | Flecs C++ | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|---:|
| 6 | 17.871 µs | 19.295 µs | 18.319 µs | **17.421 µs** | 20.152 µs | 18.987 µs | 339.577 µs |
| 8 | **18.053 µs** | 20.250 µs | 18.553 µs | 23.854 µs | 20.330 µs | 19.309 µs | 344.216 µs |
| 10 | **18.951 µs** | 22.843 µs | 19.756 µs | 19.787 µs | 23.078 µs | 20.198 µs | 348.449 µs |
| 16 | 176.976 µs | 257.070 µs | 210.504 µs | 192.009 µs | **100.852 µs** | 101.143 µs | 367.474 µs |

## 说明

- Health 和 spawn/despawn 分别重复 8 次和 32 次；换算单帧时需除以对应次数。
- 阶段测试使用独立 World，不会与完整混合帧完全相加。
- Heavy compute 用于观察重计算占主导时的循环开销，不用于 ECS 速度排名。

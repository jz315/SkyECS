# ECS Benchmark

Compare-ECS 使用语义等价的公开 API 测量六个 ECS 库。每一种正式操作都使用
针对该 workload 已认证的最快公开路径。

[GitHub Actions run 30179149116](https://github.com/jz315/SkyECS/actions/runs/30179149116)

## Comparable

| 测试 | 规模/方式 | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 实体构建 | 逐实体构建 10K | **303.595 µs** | 579.261 µs† | 674.912 µs | 587.618 µs | 926.500 µs† | 1.923 ms |
| 实体构建 | Native bulk 10K | 98.401 µs† | **13.407 µs** | 381.477 µs | 90.245 µs† | 294.031 µs | 186.128 µs† |
| 实体操作 | Spawn/despawn 1K | 53.246 µs | **47.814 µs** | 103.659 µs | 63.135 µs | 111.841 µs | 165.636 µs |
| 实体操作 | Add/remove component 1K | 108.509 µs | 109.488 µs | 161.058 µs | 142.036 µs | 222.387 µs | **74.846 µs** |
| EntityId 随机访问 | Hot 10K | 17.827 µs | **16.669 µs** | 45.692 µs | 41.246 µs | 27.244 µs | 20.631 µs |
| EntityId 随机访问 | Warm 100K | **300.135 µs** | 301.431 µs | 871.652 µs | 710.293 µs | 486.573 µs | 426.576 µs |
| Prepared 遍历 | 10K | 8.173 µs | 8.188 µs | 9.674 µs | **8.100 µs** | 13.144 µs | 18.206 µs |
| Prepared 遍历 | 100K | **81.144 µs** | 81.885 µs | 99.576 µs | 82.122 µs | 134.350 µs | 184.842 µs |
| Prepared 遍历 | 1M | 832.267 µs | 855.464 µs | 1.112 ms† | **829.999 µs** | 1.360 ms | 1.865 ms |
| 碎片遍历 | 26 × 400 | 1.160 µs | 4.459 µs | 7.750 µs | 1.265 µs | 988.421 ns | **923.137 ns** |

## Random Fragmentation

本节采用 Sander Mertens 公开的 random-fragmentation benchmark。

### Tags

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Tags | 1 | 5.939 µs | 29.843 µs† | 34.933 µs | 3.825 µs | 3.528 µs | **2.949 µs**† |
| 6 Tags | 4 | 457.790 ns | 3.445 µs† | 4.375 µs | 501.633 ns | **451.980 ns** | 382.594 µs† |
| 8 Tags | 1 | 5.698 µs | 37.142 µs† | 35.008 µs | 5.540 µs | 4.626 µs | **2.948 µs** |
| 8 Tags | 4 | 752.440 ns | 4.048 µs† | 4.373 µs | 748.551 ns | **604.581 ns** | 364.927 µs† |
| 10 Tags | 1 | 10.704 µs | 53.375 µs† | 39.432 µs | 10.495 µs | 7.026 µs | **2.963 µs** |
| 10 Tags | 4 | 1.453 µs | 6.888 µs† | 4.494 µs | 1.400 µs | **942.108 ns** | 406.671 µs |
| 10 Tags | 8 | 70.397 ns | 457.476 ns† | 318.058 ns | 103.885 ns | **65.994 ns** | 464.029 µs |
| 16 Tags | 1 | 221.658 µs | 541.044 µs | 137.323 µs† | 437.107 µs† | N/A | **2.965 µs**† |
| 16 Tags | 4 | 24.604 µs | 166.355 µs | **7.933 µs**† | 42.192 µs† | N/A | 401.200 µs† |
| 16 Tags | 8 | 1.089 µs | 16.011 µs | **464.673 ns** | 1.860 µs | N/A | 473.115 µs |

### Data Components

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Components | 1 | 48.334 µs | 70.467 µs | 61.166 µs | **47.059 µs** | 56.809 µs | 52.457 µs |
| 6 Components | 4 | **20.616 µs** | 25.189 µs | 25.338 µs | 20.979 µs | 26.255 µs | 422.057 µs |
| 8 Components | 1 | 52.118 µs | 81.304 µs | 63.895 µs | **49.654 µs** | 58.445 µs | 52.436 µs |
| 8 Components | 4 | **21.017 µs** | 25.891 µs | 25.570 µs | 21.229 µs | 26.381 µs | 413.706 µs |
| 10 Components | 1 | 71.264 µs | 103.960 µs | 89.433 µs | 66.132 µs | 69.266 µs | **52.622 µs** |
| 10 Components | 4 | **21.829 µs** | 29.306 µs | 26.490 µs | 22.290 µs | 26.616 µs | 428.837 µs |
| 10 Components | 8 | 3.481 µs | 3.957 µs | 3.375 µs | **2.697 µs** | 3.592 µs | 490.643 µs |
| 16 Components | 1 | 364.897 µs | 697.134 µs† | 348.593 µs | 610.747 µs† | N/A | **52.330 µs** |
| 16 Components | 4 | 100.506 µs | 244.489 µs | 109.055 µs | **92.563 µs**† | N/A | 426.613 µs |
| 16 Components | 8 | 7.569 µs | 20.692 µs | 6.623 µs | **6.531 µs** | N/A | 495.094 µs† |

## Gameplay Scenario

| Gameplay 项目 | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Full frame | **121.092 µs** | 147.955 µs | 211.859 µs | 140.657 µs | 189.282 µs | 328.011 µs |
| Iteration | 53.449 µs | 65.121 µs | 89.992 µs | **52.109 µs** | 96.007 µs | 203.126 µs |
| AI source lookup | 22.680 µs† | **17.282 µs**† | 37.304 µs | 29.073 µs | 21.327 µs | 31.391 µs† |
| Target Position lookup | 19.938 µs† | **18.544 µs**† | 33.259 µs† | 27.814 µs | 20.795 µs† | 20.827 µs |
| Status transition | 17.939 µs | 34.336 µs | 33.651 µs | 19.917 µs | 32.715 µs | **12.931 µs**† |
| Projectile recycle | **9.343 µs** | 13.639 µs | 26.956 µs† | 11.481 µs | 19.711 µs | 67.473 µs |

## Diagnostic

| Diagnostic | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Heavy compute | 4.108 ms | 3.499 ms | 3.557 ms | **3.469 ms** | 3.482 ms | 3.522 ms |

## 说明

数值越低越快。**粗体**表示该行最低中位数，`†` 表示 GitHub shared runner
上的波动项，`N/A` 表示没有结果。四轮不能形成完整的六引擎位置平衡区块，
因此 order bias 为 N/A。

正式报告固定为 37 行：10 行 Comparable、20 行 Random Fragmentation、
6 行 Gameplay Scenario 和 1 行 Diagnostic。Gameplay 是唯一的场景测试；
每个 phase 都推进相同的 65,536 实体、256 帧连续演化状态机，只对目标 phase
开启计时窗口。

Fixed Sequence Access 和 API candidate selection 只属于本地实验，不进入
GitHub 正式性能测试：

```bash
cargo bench -p sky_ecs_comparison --bench api_candidates \
  --features api-experiments -- fixed_sequence_access
```

正式测试命令：

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

Publisher 会在 Criterion 前执行 release contracts。原始分布、环境、
contracts 和编译器信息保留在上方链接对应的 GitHub Actions artifact 中。

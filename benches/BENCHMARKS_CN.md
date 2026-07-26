# ECS Benchmark

Compare-ECS 使用语义等价的公开 API 测量六个 ECS 库。每一种正式操作都使用
针对该 workload 已认证的最快公开路径。

[GitHub Actions run 30210139416](https://github.com/jz315/SkyECS/actions/runs/30210139416)

## Comparable

| 测试 | 规模/方式 | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 实体构建 | 逐实体构建 10K | **276.147 µs** | 580.473 µs | 812.098 µs | 746.055 µs | 883.430 µs | 1.216 ms |
| 实体构建 | 批量构建 10K | 97.557 µs† | **85.942 µs**† | 522.206 µs† | 113.279 µs† | 327.816 µs | 478.557 µs |
| 实体操作 | Spawn/despawn 1K | 52.233 µs | **46.841 µs** | 104.725 µs | 69.633 µs | 111.703 µs | 112.621 µs |
| 实体操作 | Add/remove component 1K | 107.321 µs | 109.014 µs | 164.131 µs | 135.085 µs | 219.504 µs | **51.248 µs** |
| EntityId 随机访问 | Hot 10K | 16.628 µs | **15.982 µs** | 44.384 µs | 37.963 µs | 23.440 µs | 20.246 µs |
| EntityId 随机访问 | Warm 100K | 304.531 µs | **299.879 µs** | 901.731 µs | 694.855 µs | 453.338 µs | 457.975 µs |
| Prepared 遍历 | 10K | 7.819 µs | 7.773 µs | 9.327 µs | **7.471 µs** | 12.136 µs | 17.657 µs |
| Prepared 遍历 | 100K | 77.263 µs | 78.032 µs | 93.822 µs | **75.782 µs** | 119.623 µs | 176.043 µs |
| Prepared 遍历 | 1M | **910.881 µs**† | 954.929 µs† | 1.086 ms† | 971.924 µs† | 1.258 ms | 1.794 ms |
| 碎片遍历 | 26 × 400 | 1.051 µs | 4.254 µs | 6.844 µs | 1.169 µs | 859.694 ns | **812.176 ns** |

## Random Fragmentation

本节采用 Sander Mertens 公开的 random-fragmentation benchmark。

### Tags

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Tags | 1 | 4.010 µs | 39.347 µs | 31.048 µs | 5.859 µs | **3.693 µs** | 5.123 µs |
| 6 Tags | 4 | 449.689 ns | 3.194 µs† | 3.877 µs | 745.030 ns | **431.660 ns** | 427.385 µs |
| 8 Tags | 1 | 8.305 µs | 45.969 µs | 33.454 µs | 8.458 µs | 6.955 µs | **5.127 µs** |
| 8 Tags | 4 | 839.860 ns | 4.013 µs† | 3.979 µs | 878.798 ns | **612.073 ns** | 433.407 µs |
| 10 Tags | 1 | 16.012 µs | 60.060 µs | 46.420 µs | 14.491 µs | 10.798 µs | **5.127 µs** |
| 10 Tags | 4 | 1.191 µs | 7.488 µs | 4.037 µs | 1.255 µs | **829.195 ns** | 433.651 µs |
| 10 Tags | 8 | **49.352 ns** | 428.847 ns | 283.650 ns | 110.222 ns | 59.828 ns | 509.815 µs |
| 16 Tags | 1 | 255.953 µs | 549.267 µs | 204.164 µs† | 423.656 µs† | N/A | **5.125 µs** |
| 16 Tags | 4 | 31.177 µs | 165.934 µs | **11.235 µs**† | 49.807 µs† | N/A | 436.262 µs |
| 16 Tags | 8 | 949.188 ns | 16.251 µs | **413.305 ns** | 1.669 µs | N/A | 507.019 µs |

### Data Components

| Shapes | Terms | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 6 Components | 1 | 45.023 µs | 83.185 µs | 56.020 µs | **36.267 µs** | 51.473 µs | 46.795 µs |
| 6 Components | 4 | **18.264 µs** | 26.003 µs | 22.894 µs | 18.626 µs | 23.861 µs | 456.214 µs |
| 8 Components | 1 | 51.382 µs | 89.033 µs | 63.598 µs | **41.154 µs** | 54.573 µs | 46.798 µs |
| 8 Components | 4 | **18.752 µs** | 29.739 µs† | 23.165 µs | 18.858 µs | 23.988 µs | 461.390 µs |
| 10 Components | 1 | 67.469 µs | 108.050 µs | 88.781 µs | 57.078 µs | 65.494 µs | **46.782 µs** |
| 10 Components | 4 | **19.706 µs** | 36.375 µs | 24.414 µs | 19.928 µs | 24.517 µs | 463.311 µs |
| 10 Components | 8 | 3.076 µs | 3.418 µs | 3.060 µs | **2.423 µs** | 3.364 µs | 549.839 µs |
| 16 Components | 1 | 366.952 µs† | 749.331 µs† | 395.648 µs | 778.724 µs† | N/A | **46.798 µs** |
| 16 Components | 4 | **95.813 µs** | 234.381 µs† | 109.631 µs | 96.665 µs† | N/A | 466.051 µs |
| 16 Components | 8 | 6.733 µs | 22.133 µs | 6.836 µs | **6.106 µs** | N/A | 553.427 µs |

## Gameplay Scenario

| Gameplay 项目 | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Full frame | **121.191 µs** | 146.026 µs | 212.210 µs | 138.926 µs | 183.532 µs | 326.138 µs |
| Iteration | 53.850 µs | 60.639 µs | 81.835 µs | **50.638 µs** | 86.417 µs | 208.248 µs |
| AI source lookup | 24.226 µs | **19.172 µs** | 40.636 µs† | 30.299 µs | 23.252 µs | 32.317 µs |
| Target Position lookup | 19.796 µs† | **18.266 µs** | 29.640 µs | 26.609 µs | 18.797 µs | 20.153 µs |
| Status transition | 17.851 µs | 34.407 µs | 32.262 µs | 18.769 µs | 33.265 µs | **7.696 µs** |
| Projectile recycle | **9.454 µs** | 13.621 µs | 26.935 µs | 13.391 µs | 21.690 µs | 59.700 µs† |

## Diagnostic

| Diagnostic | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| Heavy compute | 4.127 ms | 3.554 ms | 3.665 ms | **3.354 ms** | 3.598 ms | 3.477 ms |

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

Bulk construction 从统一组件列开始，并在计时区内构建各引擎的原生 batch。
本次结果正式取代已撤销的旧预构建 batch 数据。

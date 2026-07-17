# ECS Benchmark

Compare-ECS 通过公共 API 对比多个 ECS 库。各 adapter 都使用适合该
workload 的最快可复用 query、view 或 accessor。当前只测单线程，
不代表调度、并行或内存表现。

可比较 workload 会统一初始实体与组件值、最终 archetype/table 分布、查询
条件、实际读写和计时边界。每个 ECS 可以使用自身最快的官方 API 和缓存
结构；prepared 状态与输入数据可以在计时外建立，但不能提前创建本应由
计时内操作产生的目标 storage。

Prepared 随机访问对比可复用 lookup 状态（例如 Sky accessor 与 Flecs ref），
不测 prepared 状态的构造成本和内存占用。混合帧属于 scenario，heavy
compute 属于 diagnostic；这两类不参与可比较 workload 的胜负统计或整体排名。

当前 Flecs adapter 将官方 C core 与对比 adapter 静态链接，并通过直接 C ABI
调用；动态库加载、符号查询和 loader 缓存检查不在计时路径中。

## 运行

```bash
cargo compare-ecs
cargo compare-ecs -- prepared_iteration/simple_10k/sky --exact
cargo compare-ecs -- prepared_iteration/simple_10k/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- prepared_random_fragmented_iteration/random_16_components_4_terms/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` 执行六轮 Latin-square 顺序轮换，并将 Criterion 原始数据、环境信息、置信区间和跨运行中位数写入 `target/comparison-reports/`。

## 当前结果

表格记录 2026-07-17 的当前协议本地测量。大部分 Rust adapter 单元格来自
一轮完整测试；原生编译器审计后，Flecs 一列已使用 Clang/LLVM 22.1.2
重新测量。Spawn/随机 despawn 与随机 add/remove 改为固定乱序后，已对全部
adapter 重新测量；Flecs construction/mixed-spawn 单元格以及 Sky 的完整
mixed frame/heavy 单元格也在对应审计后重新测量。旧协议数据已删除。数值
取自 Criterion `median.point_estimate`；加粗表示支持该 workload 的 adapter
中的最低中位数。这是当前定向快照，不是六轮 publish 报告。

### 通用 workloads

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 批量插入 1 万 | **121.723 µs** | 205.140 µs | 307.226 µs | 237.701 µs | 263.850 µs | 282.871 µs |
| 单个插入 1 万 | **190.885 µs** | 485.219 µs | 636.250 µs | 445.943 µs | 916.928 µs | 837.098 µs |
| Prepared 遍历 1 万 | 5.308 µs | 5.354 µs | 7.932 µs | **5.136 µs** | 6.876 µs | 11.483 µs |
| Prepared 遍历 10 万 | 57.936 µs | 58.475 µs | 89.974 µs | **53.715 µs** | 70.766 µs | 119.937 µs |
| Prepared 遍历 100 万 | 1009.044 µs | 1094.400 µs | 1340.761 µs | **880.617 µs** | 1240.763 µs | 2092.052 µs |
| 碎片遍历 26 × 400 | 0.640 µs | 3.022 µs | 6.037 µs | 2.806 µs | 0.554 µs | **0.417 µs** |
| heavy compute | **2336.824 µs** | 2413.163 µs | 2421.232 µs | 2883.792 µs | 3014.084 µs | 2664.126 µs |
| Prepared 随机访问 1 万 | 16.238 µs | 11.263 µs | 30.554 µs | **7.295 µs** | 16.152 µs | 12.570 µs |
| Prepared 随机访问 10 万 | 433.720 µs | 324.000 µs | 641.115 µs | **147.942 µs** | 403.594 µs | 286.066 µs |
| Spawn/随机 despawn 1 千 | 24.756 µs | **22.927 µs** | 79.586 µs | 39.370 µs | 108.852 µs | 64.289 µs |
| 随机 add/remove component 1 千 | 55.597 µs | 67.469 µs | 97.514 µs | 78.771 µs | 134.259 µs | **27.461 µs** |
| 混合帧 | 220.182 µs | **218.611 µs** | 254.184 µs | 290.917 µs | 312.643 µs | 287.525 µs |
| 混合帧阶段：movement | 15.936 µs | **15.069 µs** | 19.819 µs | 16.919 µs | 17.109 µs | 30.737 µs |
| 混合帧阶段：health × 8 | **5.945 µs** | 17.233 µs | 38.477 µs | 17.875 µs | 27.583 µs | 56.056 µs |
| 混合帧阶段：heavy | 189.745 µs | 184.635 µs | 190.276 µs | 227.523 µs | **181.982 µs** | 184.432 µs |
| 混合帧阶段：随机访问 | 1.110 µs | 0.674 µs | 1.753 µs | **0.353 µs** | 0.986 µs | 0.695 µs |
| 混合帧阶段：结构变更 | 10.879 µs | 13.749 µs | 21.125 µs | 17.467 µs | 29.865 µs | **6.442 µs** |
| 混合帧阶段：spawn/despawn × 32 | **36.556 µs** | 42.186 µs | 122.651 µs | 59.221 µs | 198.404 µs | 155.888 µs |

### 随机碎片 workloads

测试矩阵来自 [Sander Mertens benchmark](https://gist.github.com/SanderMertens/b98ea829a1477f9b8620dd5878f707a3#file-bevy_bench-rs-L1273)。FreeCS 3.13.0 注册新 table 的成本会随已有 table 数量增长，因此它的六个 16 组件单元格记为 `N/A`；其准备阶段无法在实用的 benchmark 时间内完成。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 6 Tags，1 term | 2.983 µs | 16.778 µs | 23.681 µs | 3.003 µs | 3.531 µs | **2.583 µs** |
| 6 Tags，4 terms | **0.322 µs** | 2.120 µs | 2.869 µs | 0.343 µs | 0.427 µs | 325.795 µs |
| 8 Tags，1 term | 4.457 µs | 21.151 µs | 23.936 µs | 4.391 µs | 3.887 µs | **3.065 µs** |
| 8 Tags，4 terms | **0.415 µs** | 2.856 µs | 2.908 µs | 0.490 µs | 0.430 µs | 326.298 µs |
| 10 Tags，1 term | 6.694 µs | 35.043 µs | 24.994 µs | 7.334 µs | 5.389 µs | **2.572 µs** |
| 10 Tags，4 terms | 0.710 µs | 5.152 µs | 3.026 µs | 0.816 µs | **0.559 µs** | 332.654 µs |
| 10 Tags，8 terms | **0.031 µs** | 0.428 µs | 0.216 µs | 0.060 µs | 0.042 µs | 385.582 µs |
| 16 Tags，1 term | 390.072 µs | 1154.585 µs | 168.304 µs | 284.854 µs | N/A | **2.757 µs** |
| 16 Tags，4 terms | 26.352 µs | 168.866 µs | **5.594 µs** | 18.859 µs | N/A | 378.894 µs |
| 16 Tags，8 terms | 0.660 µs | 13.821 µs | **0.250 µs** | 0.881 µs | N/A | 424.635 µs |
| 6 数据组件，1 term | 38.215 µs | 46.801 µs | 40.150 µs | **23.870 µs** | 43.992 µs | 38.709 µs |
| 6 数据组件，4 terms | 17.265 µs | 18.763 µs | 18.532 µs | **10.658 µs** | 19.530 µs | 362.861 µs |
| 8 数据组件，1 term | 39.981 µs | 53.121 µs | 43.155 µs | **25.779 µs** | 45.487 µs | 43.931 µs |
| 8 数据组件，4 terms | 17.686 µs | 19.704 µs | 18.520 µs | **11.020 µs** | 19.187 µs | 363.577 µs |
| 10 数据组件，1 term | 53.989 µs | 77.811 µs | 66.977 µs | **37.752 µs** | 53.129 µs | 38.469 µs |
| 10 数据组件，4 terms | 18.500 µs | 22.902 µs | 19.733 µs | **12.302 µs** | 20.191 µs | 368.449 µs |
| 10 数据组件，8 terms | 2.584 µs | 2.976 µs | 2.585 µs | **1.609 µs** | 3.050 µs | 424.783 µs |
| 16 数据组件，1 term | 962.934 µs | 1567.189 µs | 801.496 µs | 604.828 µs | N/A | **41.431 µs** |
| 16 数据组件，4 terms | 135.136 µs | 275.348 µs | 218.724 µs | **103.519 µs** | N/A | 383.880 µs |
| 16 数据组件，8 terms | 5.296 µs | 19.148 µs | 6.208 µs | **4.411 µs** | N/A | 450.869 µs |

## 测试环境

- Windows 11 专业版 10.0.26200，Intel Core i7-12700F（12 核、20 逻辑处理器）。
- rustc 1.96.0（`x86_64-pc-windows-msvc`，LLVM 22.1.2）。
- Sky ECS 0.1.2、hecs 0.11.0、Bevy ECS 0.19.0、Flecs 4.1.6、
  FreeCS 3.13.0、Shipyard 0.11.5、Criterion 0.8.2。
- 每项预热 3 秒，目标测量时间至少 5 秒，共 100 个样本。
- Rust 使用 LLVM 22.1.2、`opt-level = 3`、fat LTO 和单 codegen unit；原生
  Flecs 静态库使用 Clang/LLVM `-O3 -flto -DNDEBUG`，Windows 最终链接与
  原生 LTO 由 `rust-lld` 完成。

## 说明

- Spawn/随机 despawn 在计时外生成一组固定且可复现的 1,000 个逻辑位置
  排列；每轮创建实体后按该排列删除。随机数生成与 shuffle 不计时。
- 随机 add/remove 在计时外生成两组相互独立、固定且可复现的排列：一组
  决定 Health 添加顺序，另一组决定移除顺序。每个 work item 表示一次完整的
  add/remove 周期，因此 1,000 个 work items 共执行 2,000 次结构 API 调用。
- Health 和 spawn/despawn 分别重复 8 次和 32 次；换算单帧时需除以对应次数。
- 阶段测试使用独立 World，不会与完整混合帧完全相加。
- Heavy compute 用于观察重计算占主导时的循环开销，不用于 ECS 速度排名。

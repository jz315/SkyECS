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

## Canonical workload 合同

### 最佳原生批量插入场景

`scenario_native_bulk_construction/insert_10k` 从空的、已完成 schema
准备的 World 和完全准备好的引擎原生 batch 开始。输入构造、World 构造、
注册与析构都在计时外；计时内只包含公共 bulk admission API，以及完成
1 万个四组件实体插入所必需的 iterator finalize。

| Adapter | 选定公共 API | 已准备的原生输入 |
|---|---|---|
| Sky | `World::spawn_columns` | 四个组件 `Vec` |
| hecs | `World::spawn_column_batch` | 完成的 `ColumnBatch` |
| Bevy ECS | `World::spawn_batch` | `Vec<SuiteBundle>` |
| Flecs C | `ecs_bulk_init` | 四个 C++ vector 与排序后绑定的 ID/指针对 |
| FreeCS | `World::spawn_batch` | 由 initializer 消耗的四个组件列 |
| Shipyard | `World::bulk_add_entity` | `Vec<SuiteBundle>` |

它与已退役的 `bulk_insert_10k` 不同：旧 workload 强制所有 adapter 接收
行式 bundle。旧数字会明确标记为历史 workload，不能与新的 native bulk
数字直接比较。由于准备输入是各引擎特有的，这一行属于 Scenario，不参与
Comparable 胜负统计或顺序偏差计算。

### Entity ID 与固定序列

`entity_id_random_access/{hot_10k,warm_100k}` 属于 Comparable：计时区内每次
访问都从 Entity ID 开始，并使用该引擎已经认证的最快公共 ID 查询 API，
不得替换为直接地址计划。

`scenario_fixed_sequence_access` 单独衡量稳定、重复实体序列。它报告 1 万和
10 万实体的计划构建、稳定遍历，以及构建后执行 1/4/16/64 次遍历的总成本；
报告同时给出指针 payload 与每次 traversal 的摊销时间。这类计划借用结构
冻结的 World，因此归类为 Scenario。

### 真实游戏帧

`scenario_gameplay_frame/frame` 每次执行确定性 256 帧竞技场/动作游戏
trace 中的一帧。65,536 个存活逻辑槽位分布在 32 个稳定 archetype：

| 人群 | 实体数 | 主要组件/职责 |
|---|---:|---|
| 普通移动实体 | 20,480 | Position + Velocity |
| 战斗角色 | 16,384 | 敌方伤害或友方恢复；临时 Stunned |
| AI 角色 | 8,192 | health、target slot、cooldown |
| 投射物 | 8,192 | velocity、damage、owner、64 帧生命周期 |
| 静态世界 | 8,192 | position |
| 特效 | 4,096 | position + lifetime |

每个计时帧执行 53,248 次移动更新、战斗更新、2,048 次 AI 目标直接访问、
12,288 次 lifetime 更新、128 次延迟 Stunned 移除与 128 次插入，以及
128 个投射物的 despawn/替换。状态组件真实存活 8 帧，投射物真实存活
64 帧，不存在同一帧 add/remove 抵消。场景不含矩阵求逆、人为 phase
重复或其他掩盖 ECS 成本的重计算核。

每个 adapter 都维护相同的逻辑槽位到 ECS entity handle 映射。合同测试
运行完整 256 帧，并把实体/组件计数以及 position、health、lifetime、
generation、AI lookup 的 canonical checksum 与独立于 ECS 的 reference
model 对比。总帧计时与正确性测试复用同一实现。
完整帧与 phase 测量都会在每个 256 帧 trace 后重建新 context；context 构造
和 digest 验证不计入报告时长。

`diagnostic_gameplay_phases/{iteration,ai_source_lookup,target_position_lookup,
status_transition,projectile_recycle}` 仍执行同一个持续演化的五阶段状态机，
但只累计目标 phase 的计时。Context 构造、其余四阶段、digest 检查和 trace
重置均不计时。这些行用于诊断完整帧的可能贡献；因为存在计时窗口开销，
不宣称五项与完整帧严格可加。所有 adapter 都在每帧读取 `TargetSlot`、生成
目标实体列表，再由 Position phase 消费。Flecs 的 canonical 完整帧仍只有
一次 FFI，五次调用形式仅用于诊断。

Phase diagnostics 只注册在独立的 `gameplay_phases` bench target；canonical
`comparison.rs` 与默认 publication workflow 不会执行它们。

这条原始对比固定为串行；scheduler 和 parallel 路径属于独立 benchmark
family，不混入结果。

### 最快 API 选择

API 实验与正式跨引擎比较严格分离。Sky 候选都放在
`crates/sky_ecs/benches`：`gameplay_api` 比较 iteration、AI tuple 查询、
Position 查询和完整帧，`random_access`、`entity_view`、`chunk_cost` 负责隔离
底层 API。它们只在目标机器本地运行，用 AB/BA 顺序和受控认证决定胜者。

`tools/ecs-comparison/benches/comparison.rs` 只包含已经选定的路径，不保留
候选枚举、环境变量切换或 selector；GitHub shared runner 永远不会决定生产
API。必须结合外部引擎的实验才保留在手动启用的 `api_candidates` bench。
当前 gameplay 正式路径的 AI tuple 使用 `PreparedEntityView`，目标 Position
使用 `EntityAccessor`。最新本地 AB/BA 原始记录见
[`certifications/sky-gameplay-api.windows-x86_64.2026-07-22.json`](certifications/sky-gameplay-api.windows-x86_64.2026-07-22.json)：AI 与 Position
胜者和正式路径一致；iteration function 只通过低于 2% 的中位数 fallback，
组合后未通过完整帧 gate，因此正式路径继续使用 closure。

## 最近一次公开快照（workload 修订前）

下方所有数值均来自公开的
[GitHub Actions 运行 #29695552048](https://github.com/jz315/SkyECS/actions/runs/29695552048)，
commit `e47f48163759f2e0438bcb89504908749999a416`。上传报告 artifact 的
SHA-256 为 `db5ea692ad4b32eae2614261372e2f98d0e0f9dc55bdee1e8b1d7f844543c324`。

该公开运行早于上述两项 canonical workload 修订。未受影响的
microbenchmark 行仍可作为历史证据，但旧 bulk 行不能直接改名，已退役
的 Mixed frame 也不再展示。新的 native bulk 与 Gameplay frame 数字只会
在更新后的 workflow 完成公开四轮运行后加入。

加粗表示支持该 workload 的 adapter 中的最低中位数。

### 通用 workloads

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

### 随机碎片 workloads

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

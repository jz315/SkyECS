# ECS Benchmark

Compare-ECS 使用语义等价的公开 API 测量六个 ECS 库。每一种正式操作都使用
针对该 workload 已认证的最快公开路径。

## GitHub 报告

每次完成的 GitHub 报告顶部只显示一个数据来源链接，随后固定按以下顺序展示：

1. Comparable
2. Random Fragmentation
3. Gameplay Scenario
4. Diagnostic

所有表格固定使用 `Sky`、`hecs`、`Bevy`、`Flecs C`、`FreeCS`、
`Shipyard` 列顺序。数值越低越快；粗体表示该行最低中位数，`†` 表示
noisy，`N/A` 表示没有结果。

## Comparable

Comparable 固定包含十行：

| 测试 | 规模/方式 |
|---|---|
| 实体构建 | 逐实体构建 10K |
| 实体构建 | Native bulk 10K |
| 实体操作 | Spawn/despawn 1K |
| 实体操作 | Add/remove component 1K |
| EntityId random access | Hot 10K |
| EntityId random access | Warm 100K |
| Prepared iteration | 10K |
| Prepared iteration | 100K |
| Prepared iteration | 1M |
| Fragmented iteration | 26 × 400 |

逐实体构建和 Native bulk 是同一个实体构建类别中的两种正式 Comparable
方式。

## Random Fragmentation

Random Fragmentation 是独立的正式分类。标题不添加“移植”等字样，只在说明
文字中注明其外部 benchmark 来源。

Tags 表包含 6、8 shapes 的 1/4 terms，以及 10、16 shapes 的
1/4/8 terms。Data Components 表使用相同的十个配置，共二十行。

## Gameplay Scenario

Gameplay 是唯一的场景测试，共六行：

- Full frame
- Iteration
- AI source lookup
- Target Position lookup
- Status transition
- Projectile recycle

每个 phase 都执行与完整帧相同的 65,536 实体、256 帧连续演化状态机，只对
目标 phase 开启计时窗口。

## Diagnostic

Heavy compute 是唯一的 Diagnostic 行，不计入正式比较胜负。

## 仅本地实验

Fixed Sequence Access 不进入 GitHub 报告。它只在本地测量 plan build、
steady traversal，以及把构建成本摊销到 1/4/16/64 次 traversal：

```bash
cargo bench -p sky_ecs_comparison --bench api_candidates \
  --features api-experiments -- fixed_sequence_access
```

其他 API candidates 和 AB/BA certification 同样只在本地执行。GitHub 可以
编译 candidate targets，但不能运行它们，也不能在 shared runner 上选择生产
API。

## 运行

```bash
cargo compare-ecs
cargo compare-ecs -- entity_construction/single_insert_10k/sky --exact
cargo compare-ecs -- random_fragmentation/random_16_tags_8_terms/flecs_c --exact
cargo compare-ecs -- gameplay_scenario/ai_source_lookup/sky --exact
cargo compare-ecs-publish
```

`cargo compare-ecs-publish` 会先运行 release contracts；任何合同失败都会终止
测试。完整正式报告固定为 37 行：10 行 Comparable、20 行 Random
Fragmentation、6 行 Gameplay Scenario 和 1 行 Diagnostic。原始分布、
环境、contracts 和编译器信息保留在 GitHub Actions artifact，不复制进面向
读者的报告。

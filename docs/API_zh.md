# Sky ECS API 参考

[English](API.md) · [入门教程](TUTORIAL_zh.md) · [Rustdoc](https://docs.rs/sky_ecs) · [示例](../crates/sky_ecs/examples/README_zh.md)

这是 Sky ECS 受支持公开接口的索引。各页面记录声明、约束、返回值、错误、借用与失效规则以及
复杂度。按任务学习请阅读[入门教程](TUTORIAL_zh.md)。

## Reference 页面

| 子系统 | 公开接口 |
|---|---|
| [核心实体与存储](reference/zh-CN/core.md) | `World`、`EntityId`、`Bundle`、`ColumnBundle`、`ColumnLengthMismatch`；生成、直接组件访问、迁移与销毁 |
| [实体访问](reference/zh-CN/entity-access.md) | `EntityAccessor{Mut}`、`PreparedEntityAccess{Mut}`、`PreparedEntityView`、bound view 及其构造入口 |
| [类型化查询](reference/zh-CN/queries.md) | `Query`、`QueryMut`、`PreparedQuery`、`QueryFilter`、`With`、`Without`、`Any` |
| [延迟命令](reference/zh-CN/commands.md) | `CommandBuffer` 与调度器发放的 `Commands` |
| [资源](reference/zh-CN/resources.md) | World 资源方法、`Res`、`ResMut` 与永久 `Time` 资源规则 |
| [系统](reference/zh-CN/systems.md) | `IntoSystem`、`ExclusiveSystem`、`View`、`ParView`、`EntityView`、`Local` 与 system parameter 契约 |
| [调度与时间](reference/zh-CN/scheduling.md) | `StageBuilder`、`StageLabel`、内置 stage、`FixedStep`、`Time`、报告、诊断与调度错误 |
| [动态 API](reference/zh-CN/dynamic.md) | `sky_ecs::dynamic` 安全运行时类型生成与 chunk 查询 |
| [Expert API](reference/zh-CN/expert.md) | `sky_ecs::expert` archetype、chunk、未初始化生成与原始存储契约 |
| [插件、类型与宏](reference/zh-CN/plugins-types.md) | `Plugin` 协议、组件类型注册表、`QueryData` 与 `StageLabel` derive |

## 操作索引

| 操作 | 入口 |
|---|---|
| 偶发按 ID 查组件 | `World::get` / `World::get_mut` |
| 重复访问任意 ID | `World::accessor` / `World::accessor_mut` |
| 重复访问固定 ID 序列 | `World::prepare_access` / `World::prepare_access_mut` |
| 稠密类型化遍历 | `World::query` / `World::query_mut` |
| 显式可复用类型化 query plan | `PreparedQuery` |
| 运行时选择组件 | `dynamic::DynamicQuery` |
| System/遍历期间的结构变更 | `CommandBuffer` / `Commands` |
| 调度器中的组件访问 | `View` / `ParView` |
| 调度器中按任意 ID 访问 tuple | `EntityView` |

## 接口边界

本参考覆盖 crate root 导出、`sky_ecs::dynamic`、`sky_ecs::expert`、
`sky_ecs::stage` 和 `sky_ecs::plugin`。明确排除 `__private`、sealed 实现 trait、
`pub(crate)` 项，以及没有经 expert 模块重导出的存储内部类型。Public-but-hidden 的 derive
支持项只服务生成代码，不属于手写 API。

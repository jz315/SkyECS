# Sky ECS 示例

[English](README.md)

这些示例组成一条渐进式学习路径。每个程序都能独立运行、输出确定，并通过断言检查结果。

## 核心路径

普通游戏和应用代码建议按顺序学习第 01 至 07 步。

| 步骤 | 示例 | 学习内容 | 运行命令 | 预期结果 |
|---:|---|---|---|---|
| 01 | [World](step_01_world.rs) | `World`、Bundle、直接组件访问、结构变更和带 generation 的 `EntityId` | `cargo run -p sky_ecs --example step_01_world` | 新实体有效，旧实体 ID 始终无效。 |
| 02 | [查询](step_02_queries.rs) | 只读、可变、可选、具名、携带实体 ID 和带过滤器的强类型查询 | `cargo run -p sky_ecs --example step_02_queries` | 更新 3 个移动实体，并找到 2 个有效敌人。 |
| 03 | [批量与 Chunk](step_03_batches_and_chunks.rs) | `spawn_batch`、逐实体遍历、对齐的 Chunk 切片和校验和 | `cargo run -p sky_ecs --example step_03_batches_and_chunks` | 更新 10,000 个实体，两种遍历方式结果一致。 |
| 04 | [Commands](step_04_commands.rs) | 直接结构变更和独立 `CommandBuffer` 的延迟操作 | `cargo run -p sky_ecs --example step_04_commands` | 应用一批命令，并丢弃一批已清空的命令。 |
| 05 | [系统](step_05_systems.rs) | 强类型系统参数、资源、阶段、命令、时间和固定步长 | `cargo run -p sky_ecs --example step_05_systems` | `FixedUpdate`、`Update` 和 `PostUpdate` 按稳定顺序运行。 |
| 06 | [并行遍历](step_06_parallel.rs) | 使用 `View` 与 `ParView` 完成等价更新，并进行确定性校验 | `cargo run -p sky_ecs --example step_06_parallel` | 串行与并行 World 得到相同校验和。 |
| 07 | [迷你塔防](step_07_tiny_defense.rs) | 使用延迟结构变更构建完整的分阶段游戏循环 | `cargo run -p sky_ecs --example step_07_tiny_defense` | 四帧 ASCII 输出后获胜，得分为 2，基地无损。 |

## 高级路径

第 08 和 09 步是相互独立的高级主题。完成核心路径后，在项目需要运行时类型信息或可复用模块安装时再学习。

| 步骤 | 示例 | 学习内容 | 运行命令 | 预期结果 |
|---:|---|---|---|---|
| 08 | [动态 API](step_08_dynamic.rs) | 面向工具与脚本的运行时 Bundle、读写 slot、校验和错误处理 | `cargo run -p sky_ecs --example step_08_dynamic` | 运行时查询更新组件，并拒绝重复 slot。 |
| 09 | [插件](step_09_plugin.rs) | 通过可复用 `Plugin` 安装配置和系统 | `cargo run -p sky_ecs --example step_09_plugin` | 配置影响移动结果，重复安装被拒绝。 |

Examples 用于学习 API；性能数据请查看仓库的
[Benchmark 文档](../../../benches/BENCHMARKS_CN.md)。

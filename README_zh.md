# Sky ECS

**面向 Rust 的高性能、强类型、块列式实体组件系统。**

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Sky ECS 服务于游戏与模拟中的核心热路径：高密度遍历、大规模世界、稳定帧时间，以及保持直接 Rust API 的多核并行系统。

> **在仓库内的公平公共 API 基准套件中，Sky ECS 是最快的 Rust ECS。**
> 当前记录的 Criterion 快照里，Sky 在简单遍历、批量插入和混合游戏帧中领先所测试的
> `hecs`、`bevy_ecs` 和 `flecs_ecs` 实现。性能会受工作负载、硬件、编译器和版本影响，
> 因此完整对比源码和复现命令都随仓库提供。

## 核心能力

- 块列式 Archetype 存储
- World 持有并缓存的强类型查询计划
- 实体级与 Chunk 级遍历
- 缓存并行任务、自动小负载串行回退
- 可选组件、编译期 `With` / `Without` / `Any` 过滤器
- `#[derive(QueryData)]` 命名查询项
- 世代实体、缓存结构迁移、正确的非 `Copy` 析构语义
- 强类型资源、系统、阶段、固定时间步和延迟命令
- 面向工具的安全动态 ECS，以及边界清晰的专家 API

## 快速开始

```toml
[dependencies]
sky_ecs = "0.1.1"
```

```rust
use sky_ecs::World;

#[derive(Clone, Copy)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy)]
struct Velocity { x: f32, y: f32 }

fn main() {
    let mut world = World::new();

    world.spawn_batch((0..10_000).map(|i| (
        Position { x: i as f32, y: 0.0 },
        Velocity { x: 80.0, y: 30.0 },
    )));

    world
        .query_mut::<(&mut Position, &Velocity)>()
        .for_each(|(position, velocity)| {
            position.x += velocity.x / 60.0;
            position.y += velocity.y / 60.0;
        });
}
```

大规模计算可以直接改用 `par_for_each` 或 `par_for_each_chunk`。

## 性能基准

公平对比只使用 Sky、hecs、Bevy ECS 和 Flecs 都能通过安全公共 API 表达的工作负载，
查询和 prepared 状态全部在计时区间外创建。

```bash
cargo compare-ecs
cargo compare-ecs -- fair_iteration/simple/sky --exact
```

Sky 自身的性能回归基准与跨引擎结果严格分开：

```bash
cargo bench -p sky_ecs
```

方法论与历史记录见 [`benches/BENCHMARKS_CN.md`](benches/BENCHMARKS_CN.md)。

Sky ECS 同时也是 [SkyEngine](https://github.com/jz315/SkyEngine) 使用的 ECS 基础，
在引擎中通过 `sky_engine::ecs` 重新导出。

## 许可证

MIT。

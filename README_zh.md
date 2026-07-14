# Sky ECS

[![Crates.io](https://img.shields.io/crates/v/sky_ecs.svg)](https://crates.io/crates/sky_ecs)
[![Documentation](https://docs.rs/sky_ecs/badge.svg)](https://docs.rs/sky_ecs)
[![CI](https://github.com/jz315/SkyECS/actions/workflows/ci.yml/badge.svg)](https://github.com/jz315/SkyECS/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

**面向 Rust 的高性能、强类型、块列式 ECS 库。**

[English](README.md)

Sky ECS 是一款非常快的 Rust 实体组件系统（ECS）库，在多项性能测试中表现领先。

> Sky ECS 同时也是 [SkyEngine](https://github.com/jz315/SkyEngine) 游戏引擎的内置 ECS 组件。

在主流 ECS 库横向对比测试中，Sky ECS 对标 `hecs`、`bevy_ecs`、
`flecs_ecs`、`freecs` 和 `shipyard`，在**批量插入、创建/销毁、混合帧**三项场景中耗时最低。

## 特性

- 极致性能：Archetype架构，深度的核心优化，带来极速的性能体验。
- 原生并行：内置多线程，充分利用多核 CPU 。
- 优雅易用：自然直觉的用户接口，让开发者专注于核心业务与游戏逻辑的开发。
- 动态拓展：除强类型接口外，提供完善的动态 API，便于运行时反射或与其他语言（如 C#、脚本语言）进行绑定与交互。


## 快速开始

```toml
[dependencies]
sky_ecs = "0.1.2"
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

当数据规模较大时，可将串行遍历替换为 `par_for_each` 或 `par_for_each_chunk`，充分利用多核性能。

更多用法参见 [crate 使用指南](crates/sky_ecs/README.md)、
[API 文档](https://docs.rs/sky_ecs) 和 [`examples/`](crates/sky_ecs/examples/)。

## 性能测试

Compare-ECS 使用安全公共 API，对比 Sky ECS、hecs、Bevy ECS、Flecs、
FreeCS 和 Shipyard。当前记录中，Sky 在批量插入、单个插入、
spawn/despawn 和混合帧场景中耗时最低，遍历性能与 Flecs 基本持平。

运行对比测试：

```bash
cargo compare-ecs
```

完整的 19 项测试结果、环境、版本和测量说明见
[性能测试文档](benches/BENCHMARKS_CN.md)。

## 仓库结构

```text
crates/sky_ecs/          ECS 运行时、示例与内部基准
crates/sky_ecs_derive/   QueryData 与 StageLabel 派生宏
crates/sky_type/         运行时类型标识和布局元数据
tools/ecs-comparison/    跨 ECS 对比基准
```

Sky ECS 要求 Rust 1.85 或更高版本。

## 贡献

参见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

MIT。

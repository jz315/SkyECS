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

Compare-ECS 通过共有 workload 与公共 API，对比 `hecs`、`bevy_ecs`、
Flecs、`freecs` 和 `shipyard`。

## 特性

- 极致性能：Archetype架构，深度的核心优化，带来极速的性能体验。
- 原生并行：内置多线程，充分利用多核 CPU 。
- 预解析实体访问：固定实体序列只校验和寻址一次，随后通过紧凑的直接地址计划读取或更新组件。
- 优雅易用：自然直觉的用户接口，让开发者专注于核心业务与游戏逻辑的开发。
- 动态拓展：除强类型接口外，提供完善的动态 API，便于运行时反射或与其他语言（如 C#、脚本语言）进行绑定与交互。

## 快速开始

```toml
[dependencies]
sky_ecs = "0.1.3"
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

用于 Bundle、类型查询和过滤器的元组最多支持 16 项。单个 Archetype
最多可包含 32 种不同组件；这是每个 Archetype 的存储上限，不是
`World` 可使用组件类型的总数上限。

## 文档

- [入门教程](docs/TUTORIAL_zh.md)
- [API 参考](docs/API_zh.md)
- [Rust API 文档](https://docs.rs/sky_ecs)
- [渐进式示例](crates/sky_ecs/examples/README_zh.md)

## 性能测试

下方数字是可追溯的历史公开快照，来自 commit
`e47f48163759f2e0438bcb89504908749999a416` 的
[GitHub Actions 运行 #29695552048](https://github.com/jz315/SkyECS/actions/runs/29695552048)。
旧 Mixed frame 已退役：矩阵求逆占据了绝大多数时间，无法反映 ECS
行为。替代方案是确定性的 65,536 实体、256 帧真实 Gameplay trace，
状态和投射物都有真实生命周期。新的 Gameplay 与最佳原生 bulk 数字只会
在更新后的公开四轮 workflow 完成后写入 README。

| Workload | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 旧行式 batch 插入 1 万（已退役） | 120.93 µs | 352.11 µs | 440.19 µs | **110.41 µs** | 278.08 µs | 166.75 µs |
| Prepared 遍历 1 万 | 8.12 µs | 7.83 µs | 9.35 µs | **7.69 µs** | 11.96 µs | 17.29 µs |
| Prepared 遍历 10 万 | 81.21 µs | 78.88 µs | 93.65 µs | **77.62 µs** | 120.08 µs | 174.30 µs |
| Spawn/随机 despawn 1 千 | **45.37 µs** | 46.34 µs | 103.05 µs | 67.47 µs | 112.41 µs | 107.42 µs |
| Gameplay frame（新 canonical） | 等待公开复跑 | 等待 | 等待 | 等待 | 等待 | 等待 |

完整的 workload、所有已记录项目、环境、编译器配置和测试方法见[性能测试文档](benches/BENCHMARKS_CN.md)。


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

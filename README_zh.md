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
- 优雅易用：自然直觉的用户接口，让开发者专注于核心业务与游戏逻辑的开发。
- 动态拓展：除强类型接口外，提供完善的动态 API，便于运行时反射或与其他语言（如 C#、脚本语言）进行绑定与交互。

## 快速开始

```toml
[dependencies]
sky_ecs = "0.2.0"
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

[GitHub Actions run 30705936563](https://github.com/jz315/SkyECS/actions/runs/30705936563)

| 测试 | 规模/方式 | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |
|---|---|---:|---:|---:|---:|---:|---:|
| 实体构建 | 逐实体构建 10K | **275.942 µs**† | 552.600 µs† | 771.155 µs† | 698.128 µs | 829.703 µs† | 1.161 ms |
| 实体构建 | 批量构建 10K | **42.712 µs**† | 63.621 µs† | 504.789 µs | 84.409 µs | 323.161 µs† | 466.833 µs |
| 实体操作 | Spawn/despawn 1K | **41.540 µs** | 44.792 µs | 102.800 µs | 68.277 µs | 110.843 µs | 112.659 µs |
| 实体操作 | Add/remove component 1K | 88.204 µs | 109.202 µs | 161.564 µs | 119.973 µs | 224.711 µs | **52.114 µs** |
| EntityId 随机访问 | Hot 10K | 16.445 µs | **16.147 µs** | 44.372 µs | 37.982 µs | 23.881 µs | 20.070 µs |
| EntityId 随机访问 | Warm 100K | 303.143 µs | **293.978 µs** | 858.468 µs | 670.284 µs | 445.330 µs | 439.425 µs |
| Prepared 遍历 | 10K | 7.755 µs | 7.770 µs | 9.440 µs | **7.690 µs** | 11.819 µs | 17.336 µs |
| Prepared 遍历 | 100K | 77.338 µs | 78.275 µs | 94.956 µs | **75.214 µs** | 119.823 µs | 173.370 µs |
| Prepared 遍历 | 1M | 825.249 µs† | 850.049 µs† | 954.499 µs† | **796.771 µs**† | 1.194 ms | 1.772 ms |
| 碎片遍历 | 26 × 400 | 1.045 µs | 6.854 µs | 6.840 µs | 1.125 µs | 860.278 ns | **811.752 ns** |
| Gameplay | Full frame | **113.920 µs** | 138.687 µs | 201.104 µs | 134.947 µs | 177.867 µs | 311.203 µs |

数值越低越快；`†` 表示 shared runner 上的波动项。完整 37 行报告、
workload 合同和复现命令见[性能测试文档](benches/BENCHMARKS_CN.md)。


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

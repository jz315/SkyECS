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

当数据规模较大时，可将串行遍历替换为 `par_for_each` 或 `par_for_each_chunk` 并行遍历接口，充分利用多核性能。

完整 API 文档与使用教程详见：[`crates/sky_ecs/README.md`](crates/sky_ecs/README.md)。


## Benchmark

Compare-ECS v2 只比较六个 ECS 都能通过安全公共 API 表达的单线程场景，
每个库都使用其推荐的可复用 query、view 或 accessor 稳态路径。构建与析构边界、
临时缓冲区和正确性校验均已统一，随机访问覆盖热、温、冷三档工作集。

2026-07-14 在 i7-12700F 上完成六轮顺序轮换，跨运行中位数如下：

加粗表示该项最低中位数，以及与最低值相差不超过 1% 的 Sky 结果。

| Workload | Sky | hecs | Bevy | Flecs | FreeCS | Shipyard |
|---|---:|---:|---:|---:|---:|---:|
| 批量插入 1 万 | **146.707 µs** | 242.587 µs | 287.617 µs | N/A | 261.048 µs | 157.979 µs |
| 单个插入 1 万 | **207.656 µs** | 490.967 µs | 654.194 µs | 3.433 ms | 882.560 µs | 732.709 µs |
| 稳态遍历 1 万 | **4.961 µs** | 5.096 µs | 7.693 µs | 5.146 µs | 7.784 µs | 11.029 µs |
| 稳态遍历 1 万 × 32 | **158.364 µs** | 165.264 µs | 241.443 µs | 160.181 µs | 243.145 µs | 315.386 µs |
| 稳态遍历 10 万 | **52.308 µs** | 55.084 µs | 80.748 µs | **52.013 µs** | 79.492 µs | 114.182 µs |
| 碎片遍历 26 × 400 | 0.711 µs | 2.507 µs | 5.712 µs | 1.030 µs | 4.774 µs | **0.522 µs** |
| 随机访问：热 1 万 | 15.080 µs | 128.483 µs | 35.095 µs | 314.902 µs | 14.632 µs | **10.157 µs** |
| 随机访问：温 10 万 | 221.952 µs | 1.291 ms | 511.700 µs | 3.214 ms | 224.437 µs | **154.510 µs** |
| 随机访问：冷 100 万 | 5.624 ms | 15.090 ms | 16.485 ms | 51.450 ms | 5.737 ms | **3.759 ms** |
| Spawn/despawn 1 千 | **19.573 µs** | 24.510 µs | 63.543 µs | 157.581 µs | 72.275 µs | 59.710 µs |
| Add/remove component 1 千 | 45.521 µs | 57.667 µs | 83.604 µs | 118.230 µs | 98.812 µs | **25.174 µs** |
| 诊断项：heavy compute | **1.871 ms** | **1.865 ms** | 1.870 ms | 1.873 ms | 1.871 ms | 1.870 ms |
| 混合帧 | **181.680 µs** | 195.088 µs | 238.806 µs | 223.634 µs | 208.039 µs | 200.046 µs |
| 混合帧阶段：movement | **12.596 µs** | 13.125 µs | 18.948 µs | **12.550 µs** | 18.983 µs | 26.415 µs |
| 混合帧阶段：health × 8 | **5.274 µs** | 15.234 µs | 36.988 µs | 6.260 µs | 35.346 µs | 54.606 µs |
| 混合帧阶段：heavy | **151.747 µs** | 155.301 µs | 153.572 µs | **151.016 µs** | 152.090 µs | 151.769 µs |
| 混合帧阶段：随机访问 | 2.980 µs | 6.827 µs | 1.889 µs | 16.154 µs | **0.615 µs** | 0.649 µs |
| 混合帧阶段：结构变更 | 10.869 µs | 14.916 µs | 57.986 µs | 30.062 µs | 23.769 µs | **6.382 µs** |
| 混合帧阶段：spawn/despawn × 32 | **38.367 µs** | 51.285 µs | 541.702 µs | 320.296 µs | 139.507 µs | 147.644 µs |

health 与 spawn/despawn 阶段为了降低计时噪声，分别重复执行 8 次和 32 次。
换算单帧阶段估计时需分别除以 8 和 32；独立阶段数据不会与完整混合帧完全相加。

在这份单线程快照中，Sky 领先构建、spawn/despawn 和混合帧场景，并处于遍历性能第一梯队；Shipyard 领先 prepared 随机访问和 add/remove transition。结论仅适用于表中 workload，不代表所有 ECS 使用方式。

运行一次开发测试或六轮顺序轮换的正式测试：

```bash
cargo compare-ecs
cargo compare-ecs-publish
```

详细测试规范、测量标准与场景说明见
[`benches/BENCHMARKS_CN.md`](benches/BENCHMARKS_CN.md)。

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

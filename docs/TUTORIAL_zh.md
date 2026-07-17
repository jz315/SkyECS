# Sky ECS 教程

[English](TUTORIAL.md) · [API 指南](API_zh.md)

本教程从一个小型移动模拟开始，再将它改造为由调度器驱动的 ECS 应用。

## 1. 创建项目

```toml
[dependencies]
sky_ecs = "0.1.3"
```

创建 `src/main.rs` 并导入 `World`：

```rust
use sky_ecs::World;
```

## 2. 定义组件

组件就是普通 Rust 类型，建议按职责拆分数据。

```rust
#[derive(Clone, Copy, Debug)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy, Debug)]
struct Velocity { x: f32, y: f32 }

#[derive(Clone, Copy, Debug)]
struct Enemy;
```

`Enemy` 这类 marker 组件不存储数据，但可用于查询过滤。

## 3. 创建实体

```rust
let mut world = World::new();

let player = world.spawn((
    Position { x: 0.0, y: 0.0 },
    Velocity { x: 2.0, y: 0.0 },
));

world.spawn_batch((0..1_000).map(|i| (
    Position { x: i as f32, y: 20.0 },
    Velocity { x: -1.0, y: 0.0 },
    Enemy,
)));

assert!(world.contains(player));
assert_eq!(world.entity_count(), 1_001);
```

`spawn` 返回 `EntityId`。需要定位某个具体实体时保存它；批量逻辑使用查询。

## 4. 移动匹配实体

```rust
let dt = 1.0 / 60.0;

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.x += velocity.x * dt;
        position.y += velocity.y * dt;
    });
```

查询只会访问同时拥有两个组件的实体。Rust 类型系统会拒绝重叠的可变访问。

## 5. 过滤与检查

```rust
use sky_ecs::With;

let enemies = world.query::<&Position>().filter::<With<Enemy>>();
println!("敌人数量: {}", enemies.count());

enemies.for_each_with_entity(|entity, position| {
    if position.x < 0.0 {
        println!("{entity:?} 离开了地图");
    }
});
```

`Without<T>` 用于排除 marker，`Any<(...)>` 用于 OR 过滤。如果组件是可选数据
而非过滤条件，使用 `Option<&T>`。

## 6. 延迟结构变更

不要在活跃查询中添加、移除、创建或销毁实体。调度器外可以用
`CommandBuffer` 收集这些操作：

```rust
use sky_ecs::CommandBuffer;

let mut commands = CommandBuffer::new();
commands.despawn(player);
commands.spawn((
    Position { x: 5.0, y: 5.0 },
    Velocity { x: 0.0, y: 1.0 },
));
commands.apply(&mut world);
```

系统内使用借用型 `Commands` 参数。

## 7. 添加资源与系统

资源存储 World 级单例状态。系统参数声明数据访问，调度器会安全地排列冲突系统。

```rust
use sky_ecs::{Res, ResMut, Time, Update, View};

#[derive(Default)]
struct FrameCount(u64);

fn movement(
    bodies: View<(&mut Position, &Velocity)>,
    time: Res<Time>,
) {
    bodies.for_each(|(position, velocity)| {
        position.x += velocity.x * time.delta;
        position.y += velocity.y * time.delta;
    });
}

fn count_frame(mut frame: ResMut<FrameCount>) {
    frame.0 += 1;
}

world.insert_resource(FrameCount::default());
world.stage(Update).add(movement).add(count_frame);

for _ in 0..60 {
    world.tick_with_delta(1.0 / 60.0).unwrap();
}

world.shutdown();
assert_eq!(world.get_resource::<FrameCount>().unwrap().0, 60);
```

固定频率模拟使用 `FixedUpdate` 和 `FixedStep::hz(...)`。当系统负载足够大时，
使用 `ParView<Q>` 和 `par_for_each` 执行并行遍历。

## 8. 选择正确的访问方式

- 常规遍历使用 `World::query` / `query_mut`。
- 批处理或向量化需要切片时使用 chunk 遍历。
- 只有代码必须显式持有可复用计划时才使用 `PreparedQuery`。
- 偶发的 `EntityId` 访问使用 `get` / `get_mut`。
- 结构变更应批量或延迟执行，不要与查询交错。

## 运行仓库示例

```bash
cargo run -p sky_ecs --example step_01_world
cargo run -p sky_ecs --example step_02_queries
cargo run -p sky_ecs --example step_03_batches_and_chunks
cargo run -p sky_ecs --example step_04_commands
cargo run -p sky_ecs --example step_05_systems
cargo run -p sky_ecs --example step_06_parallel
cargo run -p sky_ecs --example step_07_tiny_defense
cargo run -p sky_ecs --example step_08_dynamic
cargo run -p sky_ecs --example step_09_plugin
```

第 01 至 07 步是核心路径，第 08 至 09 步介绍高级 API。先查看
[示例索引](../crates/sky_ecs/examples/README_zh.md)，再继续阅读 [API 指南](API_zh.md) 或
[Rust API 文档](https://docs.rs/sky_ecs)。

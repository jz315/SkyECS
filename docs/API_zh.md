# Sky ECS API 指南

[English](API.md) · [教程](TUTORIAL_zh.md) · [Rustdoc](https://docs.rs/sky_ecs)

本文按常见 ECS 任务介绍公开 API。核心 ECS 类型可直接从 `sky_ecs` 导入，
也可通过 `sky_ecs::ecs` 访问；插件类型从 `sky_ecs` 或 `sky_ecs::plugin` 导入。

## API 速查

| 需求 | API |
|---|---|
| 持有 ECS 状态 | `World` |
| 创建实体 | `World::spawn`, `World::spawn_batch` |
| 读写单个实体 | `get`, `get_mut`, `has`, `insert`, `remove` |
| 遍历组件 | `query`, `query_mut` |
| 过滤 Archetype | `With`, `Without`, `Any` |
| 显式复用查询计划 | `PreparedQuery` |
| 延迟结构变更 | `Commands`, `CommandBuffer` |
| 存储单例状态 | World resource, `Res`, `ResMut` |
| 运行系统 | `View`, `ParView`, stage, `tick` |
| 安装模块 | `Plugin`, `World::install` |
| 运行时组件类型 | `sky_ecs::dynamic` |
| 低层存储集成 | `sky_ecs::expert` |

## World、实体与组件

任意 `'static` Rust 类型都可以作为组件，组件元组就是 bundle。
`EntityId` 带有 generation：实体销毁后，旧 ID 不会指向复用同一 slot 的新实体。

```rust
use sky_ecs::World;

struct Position(f32, f32);
struct Velocity(f32, f32);

let mut world = World::new();
let entity = world.spawn((Position(0.0, 0.0), Velocity(1.0, 2.0)));

assert!(world.contains(entity));
assert!(world.has::<Position>(entity));
world.get_mut::<Position>(entity).unwrap().0 += 1.0;
world.insert(entity, String::from("player"));
world.remove::<Velocity>(entity);
assert!(world.despawn(entity));
```

同一 bundle 形状的大量实体优先使用 `spawn_batch`：

```rust
world.spawn_batch((0..10_000).map(|i| (Position(i as f32, 0.0), Velocity(1.0, 0.0))));
```

`clear` 清空实体但保留资源，`entity_count` 返回存活实体数。

## 强类型查询

只读数据使用 `query`，查询中包含可变项时使用 `query_mut`。

```rust
world.query::<&Position>().for_each(|position| {
    std::hint::black_box(position);
});

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.0 += velocity.0;
        position.1 += velocity.1;
    });
```

查询项支持引用、元组和可选引用：

```rust
world
    .query::<(&Position, Option<&Velocity>)>()
    .for_each(|(position, velocity)| {
        let _ = (position, velocity);
    });
```

常用遍历方法包括 `for_each`、`for_each_with_entity`、`for_each_chunk`、
`for_each_chunk_with_entities`、`count` 和 `is_empty`。Chunk 方法会提供对齐的组件切片。

宽查询可以用 `QueryData` 定义命名项：

```rust
use sky_ecs::QueryData;

#[derive(QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
}

world.query_mut::<Movement>().for_each(|item| {
    item.position.0 += item.velocity.0;
});
```

## 过滤器

过滤器在编译期选择 Archetype，不会向查询项增加值。

```rust
use sky_ecs::{Any, With, Without};

struct Player;
struct Disabled;
struct Selected;

world
    .query::<&Position>()
    .filter::<(With<Player>, Without<Disabled>)>()
    .for_each(|position| { let _ = position; });

let visible = world
    .query::<&Position>()
    .filter::<Any<(With<Player>, With<Selected>)>>();
```

## 并行遍历

绑定查询提供 `par_for_each` 和 `par_for_each_chunk`，小规模数据会自动使用串行路径。

```rust
world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|(position, velocity)| {
        position.0 += velocity.0;
    });
```

系统中需要并行方法时使用 `ParView<Q>`，串行任务使用 `View<Q>`。

## 复用计划与随机访问

`World::query` 会在 World 中缓存匹配计划，是常规用法。当系统或提取器需要
显式持有计划，或跨 World 复用时，使用 `PreparedQuery<Q, F>`。

大量按 `EntityId` 重复查找组件时，`accessor` 和 `accessor_mut` 会预先绑定组件列。
偶发访问继续使用 `get` 和 `get_mut`。Accessor 借用 World，存活期间不能进行结构变更。

## 资源

资源是强类型单例值：

```rust
#[derive(Default)]
struct Score(u32);

world.insert_resource(Score::default());
world.get_resource_mut::<Score>().unwrap().0 += 10;
assert!(world.contains_resource::<Score>());
let score = world.remove_resource::<Score>().unwrap();
```

系统通过 `Res<T>` 和 `ResMut<T>` 请求资源。

## 结构变更与 Commands

直接的 `spawn`、`despawn`、`insert` 和 `remove` 需要可变 World。系统内使用
`Commands`，调度器外使用 `CommandBuffer`，将结构操作延迟到安全边界。

```rust
use sky_ecs::CommandBuffer;

let entity = world.spawn((Position(0.0, 0.0),));
let mut commands = CommandBuffer::new();
commands.insert(entity, Velocity(1.0, 0.0));
commands.spawn((Position(5.0, 0.0),));
commands.apply(&mut world);
```

## 系统与阶段

系统参数声明访问权限。兼容系统可并行执行，冲突系统保持稳定顺序。

```rust
use sky_ecs::{Res, Time, Update, View};

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.0 += velocity.0 * time.delta;
    });
}

world.stage(Update).add(movement);
world.tick_with_delta(1.0 / 60.0).unwrap();
world.shutdown();
```

内置顺序为 `First`、`FixedUpdate`、`PreUpdate`、`Update`、`PostUpdate`、`Last`，
固定时间步使用 `FixedStep` 配置。

## 插件与高级 API

模块可以实现 `Plugin`，安装资源和系统，再调用 `world.install(plugin)`。
同一插件类型不能重复安装。

工具、脚本和反射驱动代码使用 `sky_ecs::dynamic`；显式的低层 Archetype 或
未初始化构建集成使用 `sky_ecs::expert`。常规游戏代码应优先使用强类型 bundle 和查询。

## 继续阅读

- [教程](TUTORIAL_zh.md)
- [示例](../crates/sky_ecs/examples/)
- [Rust API 文档](https://docs.rs/sky_ecs)

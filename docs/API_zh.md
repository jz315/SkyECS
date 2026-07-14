# Sky ECS API 指南

[English](API.md) · [入门教程](TUTORIAL_zh.md) · [Rustdoc](https://docs.rs/sky_ecs)

这份指南按代码实际使用顺序讲解 API。每一节都会先给出所需类型和 `World`，
不假设你已经在其他代码块中定义了变量。

## 1. 先跑起一个完整程序

`Cargo.toml`：

```toml
[dependencies]
sky_ecs = "0.1.2"
```

`src/main.rs`：

```rust
use sky_ecs::World;

#[derive(Debug)]
struct Position { x: f32, y: f32 }

#[derive(Debug)]
struct Velocity { x: f32, y: f32 }

fn main() {
    let mut world = World::new();

    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 2.0 },
    ));

    world
        .query_mut::<(&mut Position, &Velocity)>()
        .for_each(|(position, velocity)| {
            position.x += velocity.x;
            position.y += velocity.y;
        });

    world.query::<&Position>().for_each(|position| {
        println!("{position:?}");
    });
}
```

这个程序已包含 ECS 的三个核心步骤：

1. `World::new` 创建容器。
2. `spawn` 用组件元组创建实体。
3. `query_mut` 修改组件，`query` 只读遍历。

## 2. World、组件、Bundle 和 EntityId

`World` 拥有实体、组件、资源和调度器。任意 `'static` Rust 类型都能作为组件。
传给 `spawn` 的组件元组称为 Bundle。

```rust
use sky_ecs::{EntityId, World};

struct Position(f32, f32);
struct Health(u32);
struct Player; // 零大小 marker 组件

let mut world = World::new();
let player: EntityId = world.spawn((Position(0.0, 0.0), Health(100), Player));

assert!(world.contains(player));
assert_eq!(world.entity_count(), 1);
```

`EntityId` 包含 index 和 generation。实体销毁后，旧 ID 会失效；即使底层 slot 被复用，
旧 ID 也不会意外指向新实体。

### 批量创建

大量实体具有相同 Bundle 形状时，使用 `spawn_batch`：

```rust
use sky_ecs::World;

struct Position(f32, f32);
struct Velocity(f32, f32);

let mut world = World::new();
world.spawn_batch((0..10_000).map(|i| (
    Position(i as f32, 0.0),
    Velocity(1.0, 0.0),
)));

assert_eq!(world.entity_count(), 10_000);
```

`spawn_batch` 适合不需要立即保存每个 `EntityId` 的批量导入。

## 3. 读写单个实体

已知 `EntityId` 时，使用下列方法：

| 操作 | 方法 | 失败时 |
|---|---|---|
| 检查实体 | `contains(entity)` | `false` |
| 检查组件 | `has::<T>(entity)` | `false` |
| 只读组件 | `get::<T>(entity)` | `None` |
| 修改组件 | `get_mut::<T>(entity)` | `None` |
| 添加或覆盖 | `insert(entity, value)` | `false` |
| 移除组件 | `remove::<T>(entity)` | `false` |
| 销毁实体 | `despawn(entity)` | `false` |

```rust
use sky_ecs::World;

#[derive(Debug, PartialEq)]
struct Health(u32);
struct Poison;

let mut world = World::new();
let entity = world.spawn((Health(100),));

assert_eq!(world.get::<Health>(entity), Some(&Health(100)));
world.get_mut::<Health>(entity).unwrap().0 -= 10;
assert!(world.insert(entity, Poison));
assert!(world.has::<Poison>(entity));
assert!(world.remove::<Poison>(entity));
assert!(world.despawn(entity));
assert!(world.get::<Health>(entity).is_none());
```

`insert` / `remove` / `despawn` 会改变存储结构，因此需要 `&mut World`。

## 4. 从单实体访问过渡到查询

已知 ID 且只访问一两次时用 `get`。需要处理“所有包含某些组件的实体”时用查询。

```rust
use sky_ecs::World;

struct Position { x: f32, y: f32 }
struct Velocity { x: f32, y: f32 }

let mut world = World::new();
world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
world.spawn((Position { x: 5.0, y: 0.0 },)); // 没有 Velocity

// 只匹配同时拥有 Position 和 Velocity 的实体。
world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each(|(position, velocity)| {
        position.x += velocity.x;
        position.y += velocity.y;
    });

// 查询项中需要 EntityId 时使用 with_entity 变体。
world.query::<&Position>().for_each_with_entity(|entity, position| {
    println!("{entity:?}: ({}, {})", position.x, position.y);
});
```

只读查询使用 `query`；只要查询项包含 `&mut T`，就使用 `query_mut`。

## 5. 可选组件与过滤器

`Option<&T>` 表示“匹配实体，但 T 可以不存在”。`With<T>` / `Without<T>` 表示
“只选择包含 / 不包含 T 的 Archetype”。

```rust
use sky_ecs::{With, Without, World};

struct Position(f32, f32);
struct Velocity(f32, f32);
struct Enemy;
struct Disabled;

let mut world = World::new();
world.spawn((Position(0.0, 0.0), Velocity(1.0, 0.0), Enemy));
world.spawn((Position(5.0, 0.0), Enemy));
world.spawn((Position(9.0, 0.0), Enemy, Disabled));

// 三个实体都匹配，velocity 可能为 None。
world
    .query::<(&Position, Option<&Velocity>)>()
    .filter::<With<Enemy>>()
    .for_each(|(position, velocity)| {
        let _ = (position, velocity);
    });

// 只匹配未禁用的敌人。
let active_enemies = world
    .query::<&Position>()
    .filter::<(With<Enemy>, Without<Disabled>)>();
assert_eq!(active_enemies.count(), 2);
```

OR 过滤使用 `Any<(With<A>, With<B>)>`。宽查询可以用
`#[derive(QueryData)]` 将元组改成命名字段。

## 6. Entity 遍历、Chunk 遍历和并行遍历

| 目标 | 方法 |
|---|---|
| 逐实体处理 | `for_each` |
| 同时获取 ID | `for_each_with_entity` |
| 获取对齐组件切片 | `for_each_chunk` |
| Chunk 切片加 ID 切片 | `for_each_chunk_with_entities` |
| 并行逐实体 | `par_for_each` |
| 并行 Chunk | `par_for_each_chunk` |

```rust
use sky_ecs::World;

struct Position(f32);
struct Velocity(f32);

let mut world = World::new();
world.spawn_batch((0..10_000).map(|_| (Position(0.0), Velocity(1.0))));

world
    .query_mut::<(&mut Position, &Velocity)>()
    .for_each_chunk(|(positions, velocities)| {
        for i in 0..positions.len() {
            positions[i].0 += velocities[i].0;
        }
    });

world
    .query_mut::<(&mut Position, &Velocity)>()
    .par_for_each(|(position, velocity)| {
        position.0 += velocity.0;
    });
```

常规逻辑先用 `for_each`。只有内层算法需要切片或更容易向量化时才用 Chunk。
并行有调度成本，小数据量会自动回退串行路径。

## 7. Resource：World 级单例

Resource 不属于某个实体，适合全局配置、时间、分数和共享状态。

```rust
use sky_ecs::World;

#[derive(Default)]
struct Score(u32);

let mut world = World::new();
assert!(world.insert_resource(Score::default()).is_none());
world.get_resource_mut::<Score>().unwrap().0 += 10;
assert_eq!(world.get_resource::<Score>().unwrap().0, 10);
assert!(world.contains_resource::<Score>());
let score = world.remove_resource::<Score>().unwrap();
assert_eq!(score.0, 10);
```

`insert_resource` 如果覆盖了旧值，会返回 `Some(旧值)`。

## 8. Commands：延迟结构变更

查询存活时借用了 World，不能同时调用 `spawn` / `despawn` / `insert` / `remove`。
调度器外使用 `CommandBuffer` 先记录，再统一应用：

```rust
use sky_ecs::{CommandBuffer, World};

struct Position(f32, f32);
struct Poison;

let mut world = World::new();
let entity = world.spawn((Position(0.0, 0.0),));

let mut commands = CommandBuffer::new();
commands.insert(entity, Poison);
commands.spawn((Position(5.0, 0.0),));
commands.apply(&mut world);

assert!(world.has::<Poison>(entity));
assert_eq!(world.entity_count(), 2);
```

系统内使用借用型 `Commands<'_>` 参数，调度器会在安全边界 flush。

## 9. 系统、资源与阶段

下面是一个完整可运行的调度示例：

```rust
use sky_ecs::{Res, ResMut, Time, Update, View, World};

struct Position(f32);
struct Velocity(f32);

#[derive(Default)]
struct FrameCount(u32);

fn movement(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|(position, velocity)| {
        position.0 += velocity.0 * time.delta;
    });
}

fn count_frame(mut frames: ResMut<FrameCount>) {
    frames.0 += 1;
}

fn main() {
    let mut world = World::new();
    world.insert_resource(FrameCount::default());
    world.spawn((Position(0.0), Velocity(2.0)));

    world.stage(Update).add(movement).add(count_frame);

    for _ in 0..60 {
        world.tick_with_delta(1.0 / 60.0).unwrap();
    }

    world.shutdown();
    assert_eq!(world.get_resource::<FrameCount>().unwrap().0, 60);
}
```

系统参数就是访问声明：

- `View<Q>`：串行组件查询。
- `ParView<Q>`：需要 `par_*` 方法的并行查询。
- `Res<T>` / `ResMut<T>`：只读 / 可变资源。
- `Local<T>`：系统私有、跨帧保留的状态。
- `Commands`：延迟结构变更。
- `Res<Time>`：当前帧时间。

内置阶段顺序是 `First -> FixedUpdate -> PreUpdate -> Update -> PostUpdate -> Last`。
固定频率逻辑放在 `FixedUpdate`，并用 `FixedStep::hz(...)` 配置。

## 10. 常规 API 与高级 API 的边界

| 场景 | 选择 |
|---|---|
| 常规遍历 | `World::query` / `query_mut` |
| 偶发的按 ID 访问 | `get` / `get_mut` |
| 大量重复按 ID 访问同一组件 | `accessor` / `accessor_mut` |
| 必须显式持有或跨 World 复用查询计划 | `PreparedQuery` |
| 运行时才知道组件类型 | `sky_ecs::dynamic` |
| 显式的低层 Archetype / 未初始化构建 | `sky_ecs::expert` |

不确定时，优先使用 bundle、`World::query` 和 `Commands`。这些是常规应用路径。

## 11. 继续阅读

- [从零开始的教程](TUTORIAL_zh.md)
- [渐进式可运行示例](../crates/sky_ecs/examples/README_zh.md)
- [Rustdoc 生成的逐项 API](https://docs.rs/sky_ecs)

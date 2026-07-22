# 系统与系统参数

[API 索引](../../API_zh.md) · [English](../en/systems.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## 声明

```rust
pub trait IntoSystem<Marker>: /* sealed */ + Send + 'static {}

pub trait ExclusiveSystem: 'static {
    fn init(&mut self, _world: &mut World) {}
    fn run(&mut self, world: &mut World);
    fn teardown(&mut self, _world: &mut World) {}
}

pub struct View<'w, Q, F = ()> { /* 私有字段 */ }
pub struct ParView<'w, Q, F = ()> { /* 私有字段 */ }
pub struct EntityView<'w, Q> { /* 私有字段 */ }
pub struct Res<'w, T: 'static>(/* private */);
pub struct ResMut<'w, T: 'static>(/* private */);
pub struct Local<'w, T: 'static>(/* private */);
```

## 函数系统

`IntoSystem` 自动为具有 0–16 个受支持参数的 `Send + 'static` 函数和 closure 实现。
它是 sealed trait，没有用户可调用成员。

支持的参数：

| 参数 | 约束与访问 |
|---|---|
| `View<Q, F>` | `Q` 与 `F` 声明的顺序类型化组件访问。 |
| `ParView<Q, F>` | 相同组件访问，并在串行阶段准备并行 job。 |
| `EntityView<Q>` | 面向选定任意 Entity ID 的 prepared tuple 访问。 |
| `Res<T>` | 共享资源；`T: Sync + 'static`。 |
| `ResMut<T>` | 独占资源；`T: Send + 'static`；`Time` 不可用。 |
| `Local<T>` | 以 `T::default()` 初始化的 system 私有持久状态；`T: Default + Send + 'static`。 |
| `Commands` | Invocation 私有的延迟结构变更 writer。 |
| `()` | 无访问。 |

调度器从参数类型推导组件/资源冲突。同一参数 tuple 不能声明重叠的可变能力。

## `View`

`View<'w, Q, F>` 提供：

| 声明 | Callback |
|---|---|
| `for_each<Func>(&self, f: Func)` | `Func: for<'a> FnMut(Q::Item<'a>)` |
| `for_each_with_entity<Func>(&self, f: Func)` | 额外传入 `EntityId`。 |
| `for_each_chunk<Func>(&self, f: Func)` | 传入 `Q::Chunk<'a>`。 |
| `for_each_chunk_with_entities<Func>(&self, f: Func)` | 传入行对齐的 ID 与组件 slice。 |
| `count(&self) -> usize` | 匹配活行数。 |
| `is_empty(&self) -> bool` | 是否没有匹配活行。 |
| `cached_archetype_count(&self) -> usize` | prepared 匹配 archetype 数。 |

`Q: QuerySpec`、`F: QueryFilter`。查询与缓存准备发生在调度器串行 prepare 阶段。
递归遍历同一 `View` 会 panic。

## `ParView`

`ParView<'w, Q, F>` 提供：

| 声明 | Callback |
|---|---|
| `par_for_each<Func>(&self, f: Func)` | 并行实体 item。 |
| `par_for_each_with_entity<Func>(&self, f: Func)` | 并行 EntityId 与 item。 |
| `par_for_each_chunk<Func>(&self, f: Func)` | 并行类型化 chunk slice。 |
| `par_for_each_chunk_with_entities<Func>(&self, f: Func)` | 并行对齐 ID 与 slice。 |
| `count(&self) -> usize` | 匹配活行数。 |
| `is_empty(&self) -> bool` | 是否没有匹配活行。 |
| `cached_archetype_count(&self) -> usize` | prepared 匹配 archetype 数。 |

并行 item/chunk 值必须为 `Send`，callback 必须为 `Send + Sync`。执行器可对小工作量
回退为顺序处理，执行顺序未指定。递归遍历同一 `ParView` 会 panic。

## `EntityView`

`EntityView<'w, Q>` 在普通 system 内提供 prepared 随机访问：

| 声明 | 结果 |
|---|---|
| `get(&self, entity: EntityId)` | `Q: ReadOnlyQuerySpec` 时可用；返回 `Option<Q::Item<'_>>`。 |
| `get_mut(&mut self, entity: EntityId)` | `Q: QuerySpec` 时可用；item 生命周期绑定到 view 的可变借用。 |

调度器注册 `Q` 声明的全部组件访问，并在串行 prepare 阶段刷新 route-major 列表。
死亡、过期或缺少必选组件的实体返回 `None`。只有 optional 的查询对所有活实体返回外层
`Some`，即使内部 optional 全部缺失。本参数刻意不支持 filter；过滤遍历应使用
`View<Q, F>`。

## `Res`、`ResMut` 与 `Local`

- `Res<T>` 实现 `Deref<Target = T>`；
- `ResMut<T>` 实现 `Deref<Target = T>` 与 `DerefMut`；
- `Local<T>` 实现 `Deref<Target = T>` 与 `DerefMut`；
- `Local<T>` 属于一个已注册 system，在 invocation 之间保留，并在该 system teardown 时析构。

资源规则见[资源](resources.md)。

## `ExclusiveSystem`

Exclusive system 是串行屏障并接收 `&mut World`：

| 成员 | 调用时机 |
|---|---|
| `init` | 注册或 shutdown 后，第一次运行前调用一次。 |
| `run` | 在所属 stage 的注册位置运行。 |
| `teardown` | `World::shutdown` 时按 schedule 逆序调用。 |

任意 `FnMut(&mut World) + 'static` 都实现只含 `run` 的 `ExclusiveSystem`。它不要求
`Send`，因为不会作为普通并行 wave 分发。

## 借用、错误与复杂度

- 参数引用不能逃逸一次 system invocation。
- 普通 system 不能直接结构修改 World，应使用 `Commands`。
- 缺少 `Res`/`ResMut` 时在帧 preflight 返回 `ScheduleError::MissingResource`。
- 每个已注册 system 保留查询/资源 prepared state，并在对应 World epoch 变化后刷新。
- `EntityView::get*` 在串行 route-table 准备后为 O(1)，且不分配。
- 单次遍历复杂度与对应 query 操作一致；调度 prepare 不位于组件内循环。

## 最小示例

```rust
use sky_ecs::{Commands, Local, Res, Update, View, World};

struct Position(f32);
struct Step(f32);
struct Marker;

fn advance(
    positions: View<&mut Position>,
    step: Res<Step>,
    mut runs: Local<u32>,
    mut commands: Commands,
) {
    positions.for_each(|position| position.0 += step.0);
    *runs += 1;
    if *runs == 1 {
        commands.spawn((Marker,));
    }
}

let mut world = World::new();
world.insert_resource(Step(1.0));
world.spawn((Position(0.0),));
world.stage(Update).add(advance);
world.tick_with_delta(1.0 / 60.0).unwrap();
```

## 相关 API

- [查询](queries.md)
- [延迟命令](commands.md)
- [资源](resources.md)
- [调度](scheduling.md)

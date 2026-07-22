# 类型化查询

[API 索引](../../API_zh.md) · [English](../en/queries.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## 声明

```rust
pub struct Query<'w, Q, Flt = ()> { /* 私有字段 */ }
pub struct QueryMut<'w, Q, Flt = ()> { /* 私有字段 */ }
pub struct PreparedQuery<Q, Flt = ()> { /* 私有字段 */ }

pub trait QueryFilter: /* sealed */ { /* 隐藏实现成员 */ }
pub struct With<T>(/* private */);
pub struct Without<T>(/* private */);
pub struct Any<F>(/* private */);
```

支持的 query parameter 是 `&T`、`&mut T`、`Option<&T>` 和
`Option<&mut T>`，可单独使用或组成最多 16 个组件类型的 tuple。同一查询中不能重复同一
组件类型，无论 optional 与访问模式是否不同。`#[derive(QueryData)]` 可生成具名实体级结果，
见[宏与类型](plugins-types.md)。

## World-bound 查询

```rust
pub fn World::query<Q>(&self) -> Query<'_, Q>
where
    Q: ReadOnlyQuerySpec + 'static;

pub fn World::query_mut<Q>(&mut self) -> QueryMut<'_, Q>
where
    Q: QuerySpec + 'static;
```

`Query` 只接受只读 specification；`QueryMut` 接受读写混合并独占借用 World。查询延迟
prepare，同一 `(Q, Flt)` 类型重复创建时复用 World 中的匹配元数据。

### `Query` 成员

| 声明 | Callback 契约 |
|---|---|
| `for_each_chunk<F>(&self, f: F)` | `F: for<'a> FnMut(Q::Chunk<'a>)` |
| `for_each<F>(&self, f: F)` | `F: for<'a> FnMut(Q::Item<'a>)` |
| `for_each_with_entity<F>(&self, f: F)` | `F: for<'a> FnMut(EntityId, Q::Item<'a>)` |
| `for_each_chunk_with_entities<F>(&self, f: F)` | `F: for<'a> FnMut(&'a [EntityId], Q::Chunk<'a>)` |
| `par_for_each_chunk<F>(&self, f: F)` | Chunk 为 `Send`；`F: Fn(...) + Send + Sync`。 |
| `par_for_each<F>(&self, f: F)` | Item 为 `Send`；`F: Fn(...) + Send + Sync`。 |
| `par_for_each_with_entity<F>(&self, f: F)` | 并行 EntityId 版本。 |
| `par_for_each_chunk_with_entities<F>(&self, f: F)` | 并行对齐 entity/chunk 版本。 |
| `count(&self) -> usize` | 匹配活行数。 |
| `is_empty(&self) -> bool` | 是否没有任何匹配活行。 |
| `cached_archetype_count(&self) -> usize` | 缓存的匹配 archetype 数。 |
| `filter<Flt>(self) -> Query<'w, Q, Flt>` | 一次性指定完整 filter 类型。 |

`QueryMut` 提供同名成员但接收 `&mut self`；其 `filter` 返回
`QueryMut<'w, Q, Flt>`。顺序遍历遵循稠密存储顺序，并行顺序未指定。每次 chunk callback
中的 entity slice 和组件 slice 长度相同、行完全对齐。

## `PreparedQuery`

```rust
impl<Q: QuerySpec, Flt: QueryFilter> PreparedQuery<Q, Flt> {
    pub fn new() -> Self;
    pub fn cached_archetype_count(&self) -> usize;
    pub fn count(&mut self, world: &World) -> usize;
    pub fn is_empty(&mut self, world: &World) -> bool;
}
```

`PreparedQuery` 是显式可复用 plan。它的遍历族与 World-bound query 对应，但额外接收
World：

| 声明 | World 参数 |
|---|---|
| `for_each_chunk<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each_with_entity<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `for_each_chunk_with_entities<W, F>(&mut self, world: W, f: F)` | `W: QueryWorld<Q>` |
| `par_for_each_chunk<W, F>(&mut self, world: W, f: F)` | 并行 chunk 约束。 |
| `par_for_each<W, F>(&mut self, world: W, f: F)` | 并行 item 约束。 |
| `par_for_each_with_entity<W, F>(&mut self, world: W, f: F)` | 并行 EntityId 版本。 |
| `par_for_each_chunk_with_entities<W, F>(&mut self, world: W, f: F)` | 并行对齐 chunk 版本。 |

只读 `Q` 接受 `&World` 或 `&mut World`；包含可变访问的 `Q` 必须传
`&mut World`。切换 World 或 archetype/storage 发生相关变化时，缓存自动刷新。

宽度 2–16 的 tuple query 还提供：

```rust
pub fn for_each_chunk_fn<W>(
    &mut self,
    world: W,
    function: for<'w> fn(P0::Slice<'w>, P1::Slice<'w>, /* ... */),
);

pub fn for_each_chunk_fn_with<W, State>(
    &mut self,
    world: W,
    state: &mut State,
    function: for<'w> fn(&mut State, P0::Slice<'w>, P1::Slice<'w>, /* ... */),
);
```

这两个普通函数入口把组件 slice 保持为分离参数，适用于依赖 alias 信息的计算 kernel。
每个匹配 chunk 调用一次，不接受捕获环境的 closure。

## Filter

| 类型 | 匹配规则 |
|---|---|
| `()` | 不增加过滤。 |
| `With<T>` | Archetype 包含 `T`。 |
| `Without<T>` | Archetype 不包含 `T`。 |
| `(F0, F1, ...)` | 逻辑 AND；tuple 宽度 2–16。 |
| `Any<(F0, F1, ...)>` | 逻辑 OR；tuple 宽度 2–16。 |

Filter 在 archetype 粒度工作。`QueryFilter` 是 sealed trait，外部 crate 不能实现。

## 复杂度与分配

- 首次 prepare 扫描候选 archetype 并缓存列映射；后续只扫描新增 archetype，或在需要时
  重建依赖存储布局的 chunk 元数据。
- 顺序遍历为 O(匹配 chunk 数 + 匹配实体数)，实体内循环没有动态类型查找。
- `count` 与 `is_empty` 读取 chunk 行数，不构造组件引用。
- 并行调用在存储未变化时复用 job，并可对小工作量回退为顺序执行。
- 修改组件值不使匹配缓存失效；结构变更可能使依赖存储的执行元数据失效。

## 错误与 panic

- 非法 alias、重复组件类型、不支持的 derive 形状和超出查询宽度均为编译期错误。
- World-bound borrow 阻止遍历期间的结构变更。
- 并行 callback 必须满足 `Send + Sync`，且不能依赖访问顺序。

## 最小示例

```rust
use sky_ecs::{With, World};

struct Position(f32);
struct Velocity(f32);
struct Active;

let mut world = World::new();
world.spawn((Position(1.0), Velocity(2.0), Active));

world
    .query_mut::<(&mut Position, &Velocity)>()
    .filter::<With<Active>>()
    .for_each(|(position, velocity)| position.0 += velocity.0);
```

## 相关 API

- [实体访问](entity-access.md)
- [系统 `View` 与 `ParView`](systems.md)
- [动态查询](dynamic.md)

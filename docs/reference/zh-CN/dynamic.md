# 动态 ECS API

[API 索引](../../API_zh.md) · [English](../en/dynamic.md) · [Rustdoc](https://docs.rs/sky_ecs/latest/sky_ecs/dynamic/)

模块：`sky_ecs::dynamic`

动态 API 是安全的运行时类型接口。在构造 typed slice 前，它会验证组件 identity、slot
访问模式、optionality 和 alias。

## 动态生成

```rust
pub struct ErasedComponentValue { /* 私有字段 */ }
pub struct DynamicBundle { /* 私有字段 */ }

pub trait WorldDynamicExt {
    fn spawn_dynamic(
        &mut self,
        bundle: DynamicBundle,
    ) -> Result<EntityId, DynamicSpawnError>;
}

#[derive(Debug)]
pub enum DynamicSpawnError {
    TooManyComponents { count: usize, max: usize },
    DuplicateComponent { component: ComponentType },
}
```

公开上限为 `MAX_DYNAMIC_BUNDLE_COMPONENTS`（32）和
`MAX_DYNAMIC_QUERY_SLOTS`（16）。超过任一上限会在构造 archetype 或 query plan 前返回错误。

| 类型 | 成员 |
|---|---|
| `ErasedComponentValue` | `from_typed<T: 'static>(value: T) -> Self`；`component(&self) -> ComponentType` |
| `DynamicBundle` | `new() -> Self`；`with<T: 'static>(self, value: T) -> Self`；`from_values(Vec<ErasedComponentValue>) -> Result<Self, DynamicSpawnError>`；`len`；`is_empty` |

`DynamicBundle::with` 记录值但不立即验证重复项。`from_values` 与
`WorldDynamicExt::spawn_dynamic` 拒绝重复组件 identity。成功 bundle 的值全部转移给 World；
被拒绝 bundle 的值正常析构。

## 查询构造

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicAccess { Read, Write }

pub struct DynamicQueryBuilder { /* 私有字段 */ }
pub struct DynamicQuery { /* 私有字段 */ }
```

`DynamicQuery::builder()` 与 `DynamicQueryBuilder::new()` 创建空 builder。

| Builder 成员 | 新增 slot |
|---|---|
| `read<T: 'static>(self) -> Self` | 必需共享 `T`。 |
| `write<T: 'static>(self) -> Self` | 必需独占 `T`。 |
| `optional_read<T: 'static>(self) -> Self` | 可选共享 `T`。 |
| `optional_write<T: 'static>(self) -> Self` | 可选独占 `T`。 |
| `read_component(self, ComponentType) -> Self` | 运行时类型必需读。 |
| `write_component(self, ComponentType) -> Self` | 运行时类型必需写。 |
| `optional_read_component(self, ComponentType) -> Self` | 运行时类型可选读。 |
| `optional_write_component(self, ComponentType) -> Self` | 运行时类型可选写。 |
| `build(self) -> Result<DynamicQuery, DynamicQueryError>` | 验证宽度和组件 identity 唯一性，再创建缓存查询。 |

一个组件 identity 只能占据一个 slot。

## `DynamicQuery`

| 声明 | 契约 |
|---|---|
| `slot_count(&self) -> usize` | 声明的 slot 数。 |
| `has_writes(&self) -> bool` | 是否包含可写 slot。 |
| `for_each_chunk<F>(&mut self, world: &World, f: F) -> Result<(), DynamicQueryError>` | 只读执行；包含写 slot 时拒绝。 |
| `for_each_chunk_mut<F>(&mut self, world: &mut World, f: F) -> Result<(), DynamicQueryError>` | 执行读写查询。 |

Callback 返回 `Result<(), DynamicQueryError>`；第一个 callback 错误后停止遍历。World 变化后
匹配元数据自动刷新。

## Chunk view

`DynamicQueryChunk<'w>` 与 `DynamicQueryChunkMut<'w>` 都提供 `len`、`is_empty`、
`entities`、`component(slot)`、`read<T>(slot)` 和 `optional_read<T>(slot)`。

只读 chunk 另有：

```rust
pub fn read_pair<A: 'static, B: 'static>(
    &self,
    a: usize,
    b: usize,
) -> Result<(&'w [A], &'w [B]), DynamicQueryError>;
```

可变 chunk 另有：

```rust
pub fn write<T: 'static>(&mut self, slot: usize) -> Result<&mut [T], DynamicQueryError>;
pub fn optional_write<T: 'static>(
    &mut self,
    slot: usize,
) -> Result<Option<&mut [T]>, DynamicQueryError>;
pub fn write_read<A: 'static, B: 'static>(
    &mut self,
    write_slot: usize,
    read_slot: usize,
) -> Result<(&mut [A], &[B]), DynamicQueryError>;
pub fn write_write<A: 'static, B: 'static>(
    &mut self,
    left_slot: usize,
    right_slot: usize,
) -> Result<(&mut [A], &mut [B]), DynamicQueryError>;
pub fn write_optional_read<A: 'static, B: 'static>(
    &mut self,
    write_slot: usize,
    read_slot: usize,
) -> Result<(&mut [A], Option<&[B]>), DynamicQueryError>;
```

Entity 与组件 slice 行对齐且长度都是 `len()`。多 slice 方法要求 slot 索引互异；需要从
同一可变 chunk 同时持有多个 slice 时应使用这些方法。

## `DynamicQueryError`

| Variant | 条件 |
|---|---|
| `TooManySlots { count, max }` | Builder 超过 `MAX_DYNAMIC_QUERY_SLOTS`。 |
| `DuplicateComponent { component }` | Builder 重复同一组件 identity。 |
| `InvalidSlot { slot, slot_count }` | Slot 超出查询范围。 |
| `ComponentMismatch { slot, expected, actual }` | 请求的 Rust 类型与 slot 元数据不符。 |
| `RequiresMutableWorld` | 含写 slot 的查询使用只读 executor。 |
| `RequiresWriteAccess { slot }` | 写 getter 指向只读 slot。 |
| `MissingRequiredComponent { slot }` | 必需 getter 指向当前 archetype 缺失的 optional 列。 |
| `SlotAlias { left, right }` | 多 slice 方法收到同一 slot 两次。 |

## 复杂度与分配

- Build 的唯一性验证为 O(slot 数²)，且宽度受 `MAX_DYNAMIC_QUERY_SLOTS` 限制。
- 首次/失效后的执行 prepare 匹配 archetype；chunk callback 内为常数次 slot 验证和 slice 构造。
- 遍历不逐实体分配，每个匹配 chunk 调用一次 callback。

## 最小示例

```rust
use sky_ecs::dynamic::DynamicQuery;
use sky_ecs::World;

struct Position(f32);
struct Velocity(f32);

let mut world = World::new();
world.spawn((Position(1.0), Velocity(2.0)));

let mut query = DynamicQuery::builder()
    .write::<Position>()
    .read::<Velocity>()
    .build()
    .unwrap();

query.for_each_chunk_mut(&mut world, |mut chunk| {
    let (positions, velocities) = chunk.write_read::<Position, Velocity>(0, 1)?;
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
    Ok(())
}).unwrap();
```

## 相关 API

- [运行时组件类型](plugins-types.md)
- [类型化查询](queries.md)
- [Unsafe expert API](expert.md)

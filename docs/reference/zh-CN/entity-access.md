# 实体访问

[API 索引](../../API_zh.md) · [English](../en/entity-access.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## 声明

```rust
pub struct EntityAccessor<'w, T> { /* 私有字段 */ }
pub struct EntityAccessorMut<'w, T> { /* 私有字段 */ }
pub struct PreparedEntityAccessor<T> { /* 私有字段 */ }
pub struct BoundEntityAccessor<'s, 'w, T> { /* 私有字段 */ }
pub struct BoundEntityAccessorMut<'s, 'w, T> { /* 私有字段 */ }
pub struct PreparedEntityAccess<'w, T> { /* 私有字段 */ }
pub struct PreparedEntityAccessMut<'w, T> { /* 私有字段 */ }
pub struct PreparedEntityView<Q> { /* 私有字段 */ }
pub struct BoundEntityView<'s, 'w, Q> { /* 私有字段 */ }
pub struct BoundEntityViewMut<'s, 'w, Q> { /* 私有字段 */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareAccessError {
    InvalidEntity { index: usize, entity: EntityId },
    MissingComponent { index: usize, entity: EntityId },
    DuplicateEntity {
        first_index: usize,
        duplicate_index: usize,
        entity: EntityId,
    },
}
```

`EntityAccessor<T>` 是面向任意 Entity ID 的即时通用访问路径。
`PreparedEntityAccessor<T>` 在多次 bind 之间保留单组件 route table。
`PreparedEntityAccess<T>` 预先解析一段固定序列，是重复、有序批量访问路径。
`PreparedEntityView<Q>` 按 chunk route 准备一个或多个查询组件，并在结构持续变化的帧之间
复用其分配。

## World 入口

| 声明 | 返回值 |
|---|---|
| `pub fn accessor<T: 'static>(&self) -> EntityAccessor<'_, T>` | 共享任意 ID accessor。 |
| `pub fn accessor_mut<T: 'static>(&mut self) -> EntityAccessorMut<'_, T>` | 独占任意 ID accessor。 |
| `pub fn prepare_access<T: 'static>(&self, entities: &[EntityId]) -> Result<PreparedEntityAccess<'_, T>, PrepareAccessError>` | 已验证的共享固定序列 plan。 |
| `pub fn prepare_access_mut<T: 'static>(&mut self, entities: &[EntityId]) -> Result<PreparedEntityAccessMut<'_, T>, PrepareAccessError>` | 已验证且 ID 唯一的可变固定序列 plan。 |

四种返回值都会在自身生命周期内保留对应的 World borrow，因此安全代码无法在它们存活时
执行使缓存路由或指针失效的结构变更。实现没有隐藏的 World 级 prepared cache，也不会在每个
元素上做 epoch 刷新。

## `PreparedEntityAccessor`

```rust
let mut prepared = PreparedEntityAccessor::<Position>::new();

for frame in frames {
    update_targets(frame);
    let positions = prepared.bind(&world);
    for entity in frame.targets {
        use_position(positions.get(entity)?);
    }
}
```

`bind` 和 `bind_mut` 每次都会重新取得当前 EntityRecord slice，同时复用组件 route
allocation 和已解析的列基址。因此纯 row churn 不会重建 route；切换 World、chunk
创建或退休、route 复用、tiny promotion、clear 和显式 route-table 收缩会通过
column-base epoch 触发重建。

共享 bound accessor 返回 `&T`；独占版本返回绑定到当前可变借用的 `&mut T`。每次
lookup 仍会验证 Entity generation；ID 过期或实体缺少 `T` 时返回 `None`。
`cache_stats()` 提供重建次数与 route slot 诊断。

## `PreparedEntityView`

```rust
let mut prepared = PreparedEntityView::<(&TargetSlot, &mut Cooldown)>::new();
let mut view = prepared.bind_mut(&mut world);
let (target, cooldown) = view.get_mut(entity)?;
```

`bind` 接受只读 `QuerySpec`；`bind_mut` 支持共享、可变、tuple 和 optional 参数。
World 的 column-base epoch 未变化时，bind 直接复用组件列基址；chunk 创建/退休、route
复用、tiny promotion、clear 或显式收缩 route table 后才重建，普通 row churn 不会重建。
`cache_stats()` 可查看重建次数和 route slot。每次 `get`/`get_mut` 只验证一次
generation/route，然后从同一 route 构造完整查询项。此 API 暂不支持 filter。

仅含 optional 参数的查询会区分“活实体缺少组件”和“无效实体”：
`PreparedEntityView<Option<&A>>::get` 对前者返回 `Some(None)`，对后者返回 `None`。
可变结果绑定到 bound view 当前的可变借用。

`World::route_table_stats()` 返回 live、已分配和 vacant chunk-route slot 数量。
`World::shrink_route_tables()` 只删除尾部连续 vacant slot，不会重编号 live chunk；
内部空洞仍保留给后续复用。

## `EntityAccessor`

```rust
pub fn get(&self, entity: EntityId) -> Option<&'w T>;
```

每次调用都验证 generation、解析活 chunk route，并检查该 chunk 是否有 `T`。ID
无效/过期或缺少组件时返回 `None`。构造时分配一张与活 chunk-route 槽位对应的路由表，并
一次性解析匹配组件列。

## `EntityAccessorMut`

```rust
pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T>;
```

语义与 `EntityAccessor::get` 相同。返回引用绑定到 accessor 当前的可变借用，安全代码无法
同时保留两个重叠结果。

## `PreparedEntityAccess`

| 声明 | 效果 |
|---|---|
| `pub fn len(&self) -> usize` | prepared 项数。 |
| `pub fn is_empty(&self) -> bool` | 序列是否为空。 |
| `pub fn get(&self, index: usize) -> Option<&T>` | 按 plan 索引读取；仅越界时为 `None`。 |
| `pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_` | 按原输入顺序遍历。 |

准备过程是全有或全无：所有 ID 必须有效且包含 `T`。只读 plan 可以重复同一实体。

## `PreparedEntityAccessMut`

| 声明 | 效果 |
|---|---|
| `pub fn len(&self) -> usize` | prepared 项数。 |
| `pub fn is_empty(&self) -> bool` | 序列是否为空。 |
| `pub fn get(&self, index: usize) -> Option<&T>` | 共享索引读取。 |
| `pub fn get_mut(&mut self, index: usize) -> Option<&mut T>` | 独占索引读取。 |
| `pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator + '_` | 按输入顺序共享遍历。 |
| `pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator + '_` | 按输入顺序独占遍历。 |

可变准备在 plan 可见前拒绝重复活 `EntityId`。返回的可变引用绑定到 plan 的可变借用。

## 错误

| Variant | 条件 |
|---|---|
| `InvalidEntity { index, entity }` | 输入项死亡、过期、越界或不属于此 World。 |
| `MissingComponent { index, entity }` | 实体有效但缺少 `T`。 |
| `DuplicateEntity { first_index, duplicate_index, entity }` | 可变准备中同一活 ID 出现多次。 |

验证在第一个失败输入位置停止。只读准备不会产生 `DuplicateEntity`。

## 复杂度与分配

令 `R` 为 World 的 chunk-route 槽位数，`N` 为输入长度。

| 操作 | 复杂度 / 分配 |
|---|---|
| `accessor*` 构造 | O(R + 匹配 chunk 数)，分配一张 route table。 |
| `EntityAccessor*::get*` | O(1)，不分配。 |
| `PreparedEntityAccessor::bind*` | column-base epoch 不变时为 O(1)；否则为 O(R + 匹配 chunk 数)。 |
| Bound prepared accessor `get*` | O(1)，不分配，只验证一次 entity route。 |
| `prepare_access` | O(R + 匹配 chunk 数 + N)，分配一个 boxed pointer array。 |
| `prepare_access_mut` | 期望 O(R + 匹配 chunk 数 + N)，另有用于重复检测的临时 hash table。 |
| Prepared `get*` | O(1)，不分配。 |
| Prepared `iter*` | O(N)，遍历期间不分配，也不逐项检查 entity/route/component。 |
| `PreparedEntityView::bind*` | column-base epoch 不变时 O(1)，否则 O(R + 匹配 chunk 数 × 查询宽度)。 |
| Bound entity-view `get*` | O(查询宽度)，不分配，只验证一次实体 route。 |

## 最小示例

```rust
use sky_ecs::World;

struct Position(u32);

let mut world = World::new();
let ids = [world.spawn((Position(1),)), world.spawn((Position(2),))];

let prepared = world.prepare_access::<Position>(&ids).unwrap();
assert_eq!(prepared.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 2]);
```

## 相关 API

- [核心 `World::get` 与 `World::get_mut`](core.md)
- [类型化查询](queries.md)

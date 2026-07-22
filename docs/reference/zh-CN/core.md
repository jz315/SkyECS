# 核心实体与存储

[API 索引](../../API_zh.md) · [English](../en/core.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## 声明

```rust
pub struct World {
    pub time: Time,
    // 其余字段私有
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId { /* 私有字段 */ }

pub trait Bundle: sealed::BundleSealed + 'static { /* sealed */ }
pub trait ColumnBundle: sealed::ColumnBundleSealed + 'static { /* sealed */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnLengthMismatch { /* 私有字段 */ }
```

`World` 拥有全部实体、组件、资源、插件与调度器。`EntityId` 是 World 局部的代际句柄；
相等比较同时包含槽位和 generation。

## `World` 成员

### 构造与状态

| 声明 | 效果 |
|---|---|
| `pub fn new() -> Self` | 构造空 World，并创建内置 `Time` 和内置 stage。 |
| `impl Default for World` | 等价于 `World::new()`。 |
| `pub fn is_poisoned(&self) -> bool` | 是否有延迟命令 panic 导致 World 中毒。 |
| `pub fn entity_count(&self) -> usize` | 活实体数。 |
| `pub fn archetype_count(&self) -> usize` | 当前存在的存储 archetype 数。 |
| `pub fn clear(&mut self)` | 析构全部实体和组件；保留资源和调度配置。 |

`World::clear` 会先完成存储清理，再恢复传播第一个组件析构 panic。中毒 World 仍可读取和
shutdown，但拒绝后续命令应用和 schedule tick。

### 实体与组件操作

| 声明 | 返回值 / 效果 |
|---|---|
| `pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityId` | 从组件 tuple 创建一个实体。 |
| `pub fn spawn_batch<B: Bundle>(&mut self, bundles: impl IntoIterator<Item = B>)` | 创建迭代器产出的全部实体。 |
| `pub fn spawn_columns<C: ColumnBundle>(&mut self, columns: &mut C) -> Result<(), ColumnLengthMismatch>` | 将等长组件 Vec 移入存储，并保留源 Vec 的 allocation 供复用。 |
| `pub fn contains(&self, entity: EntityId) -> bool` | 仅当槽位和 generation 对应活实体时为 `true`。 |
| `pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_` | 按稠密存储顺序遍历活 ID。 |
| `pub fn has<T: 'static>(&self, entity: EntityId) -> bool` | 活实体是否包含 `T`。 |
| `pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T>` | ID 无效/过期或缺少组件时返回 `None`。 |
| `pub fn get_mut<T: 'static>(&mut self, entity: EntityId) -> Option<&mut T>` | `get` 的可变版本。 |
| `pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) -> bool` | 原地覆盖 `T` 或迁移实体；实体无效时为 `false`。 |
| `pub fn remove<T: 'static>(&mut self, entity: EntityId) -> bool` | 删除 `T` 并迁移；实体无效或缺少 `T` 时为 `false`。 |
| `pub fn despawn(&mut self, entity: EntityId) -> bool` | 析构所有组件并使 ID 失效；无效/过期时为 `false`。 |

`spawn_columns` 在列长度不一致时具有事务性：World 和所有输入列都不改变。成功后输入
Vec 长度归零但 capacity 保留。`insert` 会先安装新值，再恢复传播旧值析构的 panic。
`remove` 和 `despawn` 会先完成存储与实体路由修复，再恢复传播析构 panic。

### 复杂度与分配

| 操作 | 复杂度 |
|---|---|
| `contains`、`has`、`get`、`get_mut` | 常数次实体路由访问，加上受 32 组件 archetype 上限约束的组件查找；不分配。 |
| `entities` | O(活实体数 + 访问的 chunk/archetype 数)；迭代器本身不收集。 |
| `spawn` | 元数据工作均摊 O(1)，另加组件移动；可能分配或扩展存储。 |
| `spawn_batch`、`spawn_columns` | O(行数 × bundle 列数)；存储按批次分配。 |
| `insert`、`remove` | 原地覆盖是有界组件工作；改变 archetype 时移动保留列且可能分配。 |
| `clear` | O(活组件值数 + 已分配存储对象数)。 |

任何结构变更都可能因 swap-remove 改变稠密遍历顺序。

## `EntityId`

| 声明 | 含义 |
|---|---|
| `pub const fn new(index: u32, generation: u32) -> Self` | 构造原始句柄；不保证在任何 World 中有效。 |
| `pub fn index(self) -> u32` | 返回可复用槽位索引。 |
| `pub fn generation(self) -> u32` | 返回用于检测过期句柄的 generation。 |

`EntityId` 不是持久化文档或网络 ID，必须由其来源 World 验证。

## `Bundle` 与 `ColumnBundle`

Sky ECS 为包含 1–16 个互异 `'static` 组件类型的 tuple 实现这两个 sealed trait。用户可以
使用这些实现，但不能自行实现。

`Bundle` 暴露以下底层成员；普通代码应使用 `World::spawn` 或 `World::spawn_batch`：

```rust
fn cached_meta() -> (Archetype, &'static [(usize, usize)]);
fn archetype() -> Archetype;
unsafe fn write(self, chunk: &mut Chunk, entity_index: usize);
unsafe fn write_fast(
    self,
    chunk: &mut Chunk,
    entity_index: usize,
    columns: &[(usize, usize)],
);
```

调用两个 unsafe writer 都要求 archetype 匹配且目标是有效的未初始化行。
`ColumnBundle` 没有公开成员。

`ColumnLengthMismatch` 成员：

| 声明 | 返回值 |
|---|---|
| `pub fn column_index(self) -> usize` | 不匹配列的从零开始索引。 |
| `pub fn expected(self) -> usize` | 第一列确定的期望行数。 |
| `pub fn actual(self) -> usize` | 不匹配列的实际行数。 |

## Panic

- 实体槽位耗尽时 panic。
- 重复组件 identity 会被拒绝。宽于 16 的 tuple bundle 没有 trait 实现；expert 构造的
  archetype 超过 32 个组件时在 `build` 中 panic。
- 用户组件析构器可以 panic，但上述所有权与修复保证仍成立。

## 最小示例

```rust
use sky_ecs::World;

struct Position(f32);

let mut world = World::new();
let entity = world.spawn((Position(1.0),));
assert_eq!(world.get::<Position>(entity).map(|p| p.0), Some(1.0));
assert!(world.remove::<Position>(entity));
assert!(!world.has::<Position>(entity));
```

## 相关 API

- [实体访问](entity-access.md)
- [查询](queries.md)
- [延迟命令](commands.md)
- [资源](resources.md)
- [Expert 存储 API](expert.md)

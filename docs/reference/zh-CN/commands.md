# 延迟命令

[API 索引](../../API_zh.md) · [English](../en/commands.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## 声明

```rust
#[derive(Default)]
pub struct CommandBuffer { /* 私有字段 */ }

pub struct Commands<'w> { /* 私有字段 */ }
```

`CommandBuffer` 是显式拥有的 buffer。`Commands` 是调度器发放的 writer，底层对应一次
system invocation 私有的 `CommandBuffer`。

## `CommandBuffer` 成员

| 声明 | 效果 |
|---|---|
| `pub fn new() -> Self` | 创建空的可复用 buffer。 |
| `pub fn is_empty(&self) -> bool` | 逻辑命令计数是否为零。 |
| `pub fn len(&self) -> usize` | 返回记录的命令数，而不是内部 batch 数。 |
| `pub fn spawn<B: Bundle>(&mut self, bundle: B)` | 延迟 spawn；连续相同 bundle 类型会合并为 batch。 |
| `pub fn despawn(&mut self, entity: EntityId)` | 延迟 despawn。 |
| `pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T)` | 延迟插入或覆盖。 |
| `pub fn remove<T: 'static>(&mut self, entity: EntityId)` | 延迟删除组件。 |
| `pub fn insert_resource<R: 'static>(&mut self, resource: R)` | 延迟插入资源。 |
| `pub fn remove_resource<R: 'static>(&mut self)` | 延迟删除资源。 |
| `pub fn apply(&mut self, world: &mut World)` | 应用 active prefix，并将 buffer 恢复为空。 |
| `pub fn clear(&mut self)` | 析构待处理 payload，但保留可复用 capacity。 |

同一实体的命令会合并为每种组件的最终状态，再执行结构迁移。注册与应用顺序保持确定性。

## `Commands` 成员

`Commands<'w>` 提供同名的 `is_empty`、`len`、`spawn`、`despawn`、`insert`、
`remove`、`insert_resource` 和 `remove_resource`。调度器安全边界增加：

- `spawn` 要求 `B: Bundle + Send`；
- `insert` 要求 `T: Send + 'static`；
- `insert_resource` 要求 `R: Send + 'static`。

它不暴露 `apply` 或 `clear`；flush 边界由调度器拥有，system buffer 按注册顺序应用。

## Panic 与中毒契约

`apply` 可能运行用户析构器和其他用户代码。若 panic 逃逸：

- 未访问命令被丢弃，每个自有 payload 最多析构一次；
- buffer 恢复为空的可观察状态；
- 任意部分变更无法通用回滚，因此 World 被标记为中毒；
- 中毒 World 拒绝后续命令应用和 schedule tick，但仍可读取并 shutdown。

向已中毒 World 应用命令会 panic。`World::is_poisoned` 可查询该状态。

## 复杂度与分配

- 记录操作均摊 O(1)，`apply` 或 `clear` 后复用内部槽位。
- 连续同类型 spawn 与连续实体命令使用批量存储。
- `apply` 为 O(命令数 + 受影响组件数据量)；合并可能减少 archetype 迁移次数。
- `clear` 为 O(自有 payload 数)，并保留 buffer capacity。

## 最小示例

```rust
use sky_ecs::{CommandBuffer, World};

struct Health(u32);

let mut world = World::new();
let entity = world.spawn((Health(100),));
let mut commands = CommandBuffer::new();
commands.insert(entity, Health(50));
commands.apply(&mut world);

assert_eq!(world.get::<Health>(entity).map(|h| h.0), Some(50));
```

## 相关 API

- [核心实体操作](core.md)
- [系统与系统参数](systems.md)
- [调度与 flush 边界](scheduling.md)

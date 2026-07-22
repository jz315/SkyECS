# 资源

[API 索引](../../API_zh.md) · [English](../en/resources.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`

## World 资源成员

```rust
pub fn World::insert_resource<R: 'static>(&mut self, resource: R) -> Option<R>;
pub fn World::get_resource<R: 'static>(&self) -> Option<&R>;
pub fn World::get_resource_mut<R: 'static>(&mut self) -> Option<&mut R>;
pub fn World::contains_resource<R: 'static>(&self) -> bool;
pub fn World::remove_resource<R: 'static>(&mut self) -> Option<R>;
```

资源以 Rust `TypeId` 为键，每个 Rust 类型最多保存一个值。`insert_resource` 返回被替换
值，`remove_resource` 返回被删除值；类型不存在时 lookup 返回 `None`。

`Time` 是永久内置 World 状态：

- `contains_resource::<Time>()` 始终为 `true`；
- `get_resource::<Time>()` 和 `get_resource_mut::<Time>()` 指向 `World::time`；
- `insert_resource::<Time>` 和 `remove_resource::<Time>` 会 panic。

独占代码可在 tick 之间修改 `Time`。普通 scheduled system 可以请求 `Res<Time>`，但不能
请求 `ResMut<Time>`。

## System 资源参数

```rust
pub struct Res<'w, T: 'static>(/* private */);
pub struct ResMut<'w, T: 'static>(/* private */);
```

| 参数 | 约束 | 访问 |
|---|---|---|
| `Res<'w, T>` | `T: Sync + 'static` | 共享；实现 `Deref<Target = T>`。 |
| `ResMut<'w, T>` | `T: Send + 'static` | 独占；实现 `Deref` 与 `DerefMut`。 |

调度器在一帧运行前验证资源可用性。参数缺失会在时间推进和任何 system 运行前返回
`ScheduleError::MissingResource { system, resource }`。资源读写冲突参与 wave 构建。

## 延迟资源操作

`CommandBuffer` 与 `Commands` 都提供 `insert_resource`、`remove_resource`，并遵循
[延迟命令](commands.md)中的 flush 与 World 中毒规则。

## 复杂度与失效

- 直接资源操作使用类型键 hash map，期望 O(1)。
- 插入/删除可能分配或释放；lookup 不分配。
- 插入/删除递增 resource epoch。Prepared system resource pointer 仅在同一 World 且
  resource epoch 未变时复用。
- 替换资源会使下次 invocation 前的旧调度元数据失效；普通 Rust borrow 阻止直接引用跨越
  该变更。

## 最小示例

```rust
use sky_ecs::{Res, Update, World};

struct Gravity(f32);

fn read_gravity(gravity: Res<Gravity>) {
    assert_eq!(gravity.0, 9.8);
}

let mut world = World::new();
world.insert_resource(Gravity(9.8));
world.stage(Update).add(read_gravity);
world.tick_with_delta(1.0 / 60.0).unwrap();
```

## 相关 API

- [系统](systems.md)
- [调度](scheduling.md)
- [`Time`](scheduling.md#time)

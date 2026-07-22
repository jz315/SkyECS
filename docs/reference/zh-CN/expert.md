# Expert 存储 API

[API 索引](../../API_zh.md) · [English](../en/expert.md) · [Rustdoc](https://docs.rs/sky_ecs/latest/sky_ecs/expert/)

模块：`sky_ecs::expert`

该模块直接暴露存储 invariant。Gameplay 和普通工具应优先使用类型化 API 或
[`dynamic`](dynamic.md)。

## 导出

```rust
pub use /* ... */ {
    Archetype,
    ArchetypeBuilder,
    Chunk,
    ComponentType,
    ComponentTypeInfo,
    PreparedQuery,
};

pub fn create_archetype() -> ArchetypeBuilder;
pub fn component_type<T: 'static>() -> ComponentType;
pub fn register_component_type(name: &str, size: usize, align: usize) -> ComponentType;
pub fn interned_archetype_count() -> usize;

pub trait WorldExpertExt {
    unsafe fn spawn_uninit(&mut self, archetype: Archetype) -> EntityId;
}
```

`ComponentType` 与 `PreparedQuery` 的契约和 crate root 导出完全相同。

## `ArchetypeBuilder` 与 `Archetype`

| 声明 | 效果 |
|---|---|
| `create_archetype() -> ArchetypeBuilder` | 开始空组件 signature。 |
| `ArchetypeBuilder::add_component(self, ty: ComponentType) -> Self` | 增加运行时类型元数据。 |
| `ArchetypeBuilder::add_rust_component<T: 'static>(self) -> Self` | 增加 `component_type::<T>()`。 |
| `ArchetypeBuilder::build(self) -> Archetype` | 排序、验证并在进程中 intern signature。 |
| `Archetype::id(&self) -> usize` | 由 interned metadata 派生的进程局部 identity。 |
| `interned_archetype_count() -> usize` | 进程保留的 signature 数。 |

`Archetype` 实现 `Copy + Eq + Hash`，并 deref 到不可变元数据，暴露 `components`、
`alignment`、`has_component` 与 `query_component_index`。Interned metadata 有意不回收。
Build 拒绝重复 identity 和超过 32 个组件的 signature。

## `Chunk`

```rust
pub struct Chunk {
    pub entity_count: usize,
    pub max_entity_count: usize,
    pub archetype: Archetype,
    // 私有 allocation 元数据
}
```

| 声明 | 契约 |
|---|---|
| `Chunk::new(archetype: Archetype) -> Self` | 使用默认 layout policy 分配 chunk。 |
| `is_full(&self) -> bool` | `entity_count == max_entity_count`。 |
| `is_empty(&self) -> bool` | `entity_count == 0`。 |
| `unsafe add_entity(&mut self, entity: EntityId) -> Option<usize>` | 预留逻辑行；观察或析构前，调用者必须初始化全部组件。 |
| `column_ptr(&self, component_index: usize) -> *mut u8` | 原始列首地址；组件索引必须有效。 |
| `data_ptr(&self) -> *mut u8` | 原始 allocation 首地址；全 ZST layout 为对齐 dangling 地址。 |
| `component_ptr(&self, component_index: usize, entity_index: usize) -> *mut u8` | 行指针；实体行越界返回 null，但组件索引仍必须有效。 |
| `get_entity_as_ptr(&self, index: usize) -> *const u8` | 第一个组件的行指针；空 archetype/越界返回 null。 |
| `entity_id(&self, entity_index: usize) -> Option<EntityId>` | 逻辑行中的 ID。 |

原始指针函数不创建 Rust 引用，也不验证类型 identity、初始化状态、alias 或大部分组件索引
前置条件。

## `WorldExpertExt::spawn_uninit`

```rust
unsafe fn spawn_uninit(&mut self, archetype: Archetype) -> EntityId;
```

返回实体已经注册进 World。在查询、移动、删除或析构它前，调用者必须恰好一次初始化
`archetype` 描述的每个组件槽位。违反契约可能导致无效读取或对未初始化数据调用析构器。

## 生命周期、分配与复杂度

- Archetype handle 和组件类型 handle 引用进程生命周期 interned metadata。
- 新 signature/type 可能分配并永久保留元数据。
- 复用 archetype 是 registry lookup；handle 相等与 hash 为 O(1)。
- Chunk 按 layout 分配；全 ZST layout 使用对齐 dangling 组件地址和逻辑实体存储。
- 指针 accessor 为 O(1) 且不分配。

## 相关 API

- [核心安全 API](core.md)
- [动态安全 API](dynamic.md)
- [组件类型注册表](plugins-types.md)

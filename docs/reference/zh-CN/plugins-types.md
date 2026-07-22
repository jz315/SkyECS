# 插件、组件类型与 Derive 宏

[API 索引](../../API_zh.md) · [English](../en/plugins-types.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`、`sky_ecs::plugin`

## 插件协议

```rust
pub type PluginResult = Result<(), PluginError>;

pub struct PluginError {
    pub plugin: &'static str,
    pub message: String,
}

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn install(self, world: &mut World) -> PluginResult
    where
        Self: Sized;
}

#[derive(Default)]
pub struct PluginRegistry { /* 私有字段 */ }
```

`PluginError::new(plugin, message)` 构造错误；该类型实现 `Display + Error`。

`PluginRegistry` 成员：

| 声明 | 效果 |
|---|---|
| `contains<P: 'static>(&self) -> bool` | 是否已记录插件类型 `P`。 |
| `get<P: 'static>(&self) -> Option<&'static str>` | `P` 的已安装名称。 |
| `insert<P: 'static>(&mut self, name: &'static str) -> Result<(), &'static str>` | 记录 `P`，重复时返回已有名称。 |

World 入口：

| 声明 | 效果 |
|---|---|
| `install<P: Plugin + 'static>(&mut self, plugin: P) -> PluginResult` | 运行安装，再记录具体插件类型；重复安装返回错误。 |
| `has_plugin<P: 'static>(&self) -> bool` | 检查当前 World 的安装记录。 |
| `require_plugin<P: 'static>(&self, plugin: &'static str) -> PluginResult` | 依赖检查；错误使用调用者提供的插件名。 |

插件安装不是事务：失败的 `Plugin::install` 可能已经改变 World。Registry 只记录安装
identity，不保存配置或 capability。

## 组件类型元数据

```rust
pub type ComponentType = sky_type::Type;
pub type ComponentTypeInfo = sky_type::TypeInfo;

pub fn component_type<T: 'static>() -> ComponentType;
pub fn register_component_type(name: &str, size: usize, align: usize) -> ComponentType;
pub fn component_type_by_name(name: &str) -> Option<ComponentType>;
pub fn component_type_by_rust_type<T: 'static>() -> Option<ComponentType>;
pub fn registered_component_types() -> Vec<ComponentType>;
```

`ComponentType` 是可复制的进程 interned handle，并 deref 到：

```rust
pub struct ComponentTypeInfo {
    pub size: usize,
    pub align: usize,
    pub name: String,
    pub drop_fn: Option<unsafe fn(*mut u8)>,
    // Rust TypeId 私有
}
```

| 成员 | 含义 |
|---|---|
| `id(&self) -> usize` | 用于相等与 hash 的进程局部 metadata identity。 |
| `needs_drop(&self) -> bool` | 是否存在 erased destructor。 |
| `drop_fn(&self) -> Option<unsafe fn(*mut u8)>` | Erased destructor；调用者必须传入该注册类型的有效已初始化值。 |
| `rust_type_id(&self) -> Option<TypeId>` | Typed 注册的 Rust identity；opaque 动态类型为 `None`。 |

`component_type::<T>()` 注册或返回精确 Rust layout/drop metadata。
`component_type_by_rust_type::<T>()` 只查询、不注册。
`register_component_type` 创建没有析构器的 opaque 运行时 metadata。

名称为空、alignment/layout 非法、名称以不兼容 layout 重用、opaque 与 Rust 注册冲突时
会 panic。Metadata 保留到进程结束。`registered_component_types` 分配一个无序快照 Vec。

## `#[derive(QueryData)]`

```rust
#[derive(sky_ecs::QueryData)]
struct Movement<'w> {
    position: &'w mut Position,
    velocity: &'w Velocity,
    health: Option<&'w Health>,
}
```

要求：

- 具有命名且非空字段的 struct；
- 恰好一个无 bounds 的 lifetime parameter；
- 没有 type/const parameter 或 `where` clause；
- 1–16 个使用该 lifetime 的 `&T`、`&mut T`、`Option<&T>` 或
  `Option<&mut T>` 字段；
- 不重复组件类型。

派生类型可作为 `Q`。Chunk 级方法仍返回底层组件 slice tuple；实体级方法返回具名 struct。

## `#[derive(StageLabel)]`

```rust
#[derive(sky_ecs::StageLabel)]
struct Physics;
```

输入必须是不带泛型且没有 `where` clause 的 unit struct。Derive 实现
`sky_ecs::StageLabel`，默认 stage 名是 Rust 类型名。

## 复杂度与线程安全

- 组件 handle 相等/hash 与 metadata 访问为 O(1)。
- 首次注册可能获取全局 registry 写锁并分配；重复 typed lookup 先使用 thread-local cache。
- Plugin registry 使用 World 局部 type-keyed hash map，期望 O(1)。

## 相关 API

- [动态 API](dynamic.md)
- [Expert API](expert.md)
- [调度](scheduling.md)
- [查询](queries.md)

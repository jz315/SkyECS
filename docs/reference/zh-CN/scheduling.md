# 调度、Stage 与时间

[API 索引](../../API_zh.md) · [English](../en/scheduling.md) · [Rustdoc](https://docs.rs/sky_ecs)

模块：`sky_ecs`、`sky_ecs::stage`

## Stage label

```rust
pub trait StageLabel: Send + Sync + 'static {
    fn name() -> &'static str { /* 默认 type_name::<Self>() */ }
}

pub struct First;
pub struct FixedUpdate;
pub struct PreUpdate;
pub struct Update;
pub struct PostUpdate;
pub struct Last;
```

内置顺序为 `First`、`FixedUpdate`、`PreUpdate`、`Update`、`PostUpdate`、`Last`。
内置 label 都是可复制零大小值。自定义 unit struct 可使用 `#[derive(StageLabel)]`。

## World 调度成员

| 声明 | 效果 |
|---|---|
| `pub fn stage<L: StageLabel>(&mut self, label: L) -> StageBuilder<'_>` | 获取已安装 stage；不存在时 panic。 |
| `pub fn try_stage<L: StageLabel>(&mut self, label: L) -> Result<StageBuilder<'_>, ScheduleBuildError>` | 可失败的已安装 stage lookup。 |
| `pub fn insert_stage_after<Anchor, L>(&mut self, anchor: Anchor, label: L) -> Result<StageBuilder<'_>, ScheduleBuildError>` | 在已有 anchor 后安装自定义 typed stage。 |
| `pub fn schedule_diagnostics(&mut self) -> ScheduleDiagnostics` | 必要时编译 dirty wave，并返回自有快照；不运行 system。 |
| `pub fn tick(&mut self) -> Result<TickReport, ScheduleError>` | 使用 wall-clock delta 推进；第一次 delta 为零。 |
| `pub fn tick_with_delta(&mut self, delta: f32) -> Result<TickReport, ScheduleError>` | 同一值作为 frame 与 raw delta。 |
| `pub fn tick_with_frame_delta(&mut self, frame_delta: f32, raw_delta: f32) -> Result<TickReport, ScheduleError>` | 分离钳制/缩放后的模拟输入与真实时间。 |
| `pub fn shutdown(&mut self)` | 按逆序 teardown system。 |

递归 tick 或在 tick 中修改/检查 schedule 会 panic；中毒 World 的 tick 也会 panic。帧
preflight 发现资源缺失时返回错误，不推进时间，也不运行 system。

两个显式 delta 输入都会归一化为有限非负值；负数、NaN 与无穷大变为零。乘以
`Time::time_scale` 后，scaled frame delta 会再次归一化。

## `StageBuilder`

```rust
pub struct StageBuilder<'a> { /* 私有字段 */ }
```

| 声明 | 效果 |
|---|---|
| `fixed(&mut self, step: FixedStep) -> Result<&mut Self, ScheduleBuildError>` | 配置固定步长。 |
| `parallel_wave_min_systems(&mut self, minimum: usize) -> Result<&mut Self, ScheduleBuildError>` | 交给 Rayon 的兼容 wave 最小宽度；默认 3、最小 2，`usize::MAX` 强制串行。 |
| `add<Func, Marker>(&mut self, system: Func) -> &mut Self` | 以 Rust 类型名添加普通 typed system。 |
| `add_named<Func, Marker>(&mut self, name: impl Into<String>, system: Func) -> &mut Self` | 添加命名普通 system。 |
| `add_exclusive<S: ExclusiveSystem>(&mut self, system: S) -> &mut Self` | 添加串行 full-World 屏障。 |
| `add_exclusive_named<S: ExclusiveSystem>(&mut self, name: impl Into<String>, system: S) -> &mut Self` | 命名 exclusive 版本。 |

显式空 system 名会 panic。兼容普通 system 组成 wave；访问冲突和 exclusive system 切分
并行执行。延迟命令在调度边界按注册顺序 flush。

## 固定步长

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedOverflow { Drop, Carry }

#[derive(Clone, Copy, Debug)]
pub struct FixedStep { /* 私有字段 */ }
```

| 声明 | 契约 |
|---|---|
| `FixedStep::hz(hz: u32) -> Self` | `hz == 0` 时 panic。 |
| `FixedStep::try_hz(hz: u32) -> Result<Self, ScheduleBuildError>` | 验证频率后取倒数。 |
| `FixedStep::seconds(seconds: f64) -> Result<Self, ScheduleBuildError>` | 要求有限且 `seconds > 0`。 |
| `max_substeps(self, max_substeps: u32) -> Self` | 每帧预算；零会 panic。 |
| `overflow(self, overflow: FixedOverflow) -> Self` | 丢弃或保留 backlog。 |
| `step_seconds(self) -> f64` | 配置的步长秒数。 |
| `substep_limit(self) -> u32` | 配置的预算。 |

构造器默认 8 个 substep 和 `FixedOverflow::Drop`。`FixedUpdate` 初始为 60 Hz；第一次显式
设置替换默认值，后续等价设置幂等，不同设置返回 `ConflictingFixedStep`。

## `Time`

```rust
#[derive(Debug, Clone)]
pub struct Time {
    pub delta: f32,
    pub frame_delta: f32,
    pub raw_delta: f32,
    pub elapsed: f32,
    pub raw_elapsed: f32,
    pub frame_count: u64,
    pub fixed_alpha: f32,
    pub time_scale: f32,
}
```

`delta()`、`frame_delta()`、`raw_delta()`、`elapsed()`、`raw_elapsed()`、
`fixed_alpha()` 返回对应字段。

- `delta` 是当前 stage delta：fixed stage 内为固定步长，其余为 `frame_delta`；
- `frame_delta` 与 `elapsed` 受 `time_scale` 影响；
- `raw_delta` 与 `raw_elapsed` 不受影响；
- `fixed_alpha` 是内置 `FixedUpdate` 的已累计步长比例；
- `Time::default()` 将计数/delta 置零，`time_scale` 置 1。

每个 World 都把永久 `Time` 同时暴露为 `world.time` 与内置资源。
`Time` 是 `Sync` 但有意不是 `Send`；普通 system 只能获得共享 `Res<Time>`。

## 报告与诊断

`TickReport` 字段：

| 字段 | 类型 |
|---|---|
| `frame` | `u64` |
| `systems_run`、`waves_run`、`parallel_waves_run`、`sequential_waves_run`、`fixed_substeps` | `u32` |
| `dropped_fixed_time` | `f64` |

Schedule diagnostics：

```rust
pub struct ScheduleDiagnostics { pub stages: Vec<StageDiagnostics> }
pub struct StageDiagnostics {
    pub name: &'static str,
    pub fixed: Option<FixedStageDiagnostics>,
    pub parallel_wave_min_systems: usize,
    pub segments: Vec<StageSegmentDiagnostics>,
}
pub struct FixedStageDiagnostics {
    pub step_seconds: f64,
    pub max_substeps: u32,
    pub overflow: FixedOverflow,
    pub accumulated_seconds: f64,
}
pub struct StageSegmentDiagnostics {
    pub waves: Vec<Vec<SystemDiagnostics>>,
    pub exclusive_after: Option<String>,
}
pub struct SystemDiagnostics {
    pub registration_index: usize,
    pub name: String,
    pub access: SystemAccessDiagnostics,
    pub commands: CommandDiagnostics,
}
```

`CommandDiagnostics` 的 `last_enqueued`、`last_applied`、`last_discarded` 为 `usize`，
`total_enqueued`、`total_applied`、`total_discarded` 为 `u64`。

`SystemAccessDiagnostics` 暴露组件/资源读写名称 Vec 和 `uses_commands: bool`；
`conflict_reason(&self, other: &Self) -> Option<String>` 返回第一个类型化组件或资源冲突。

## 错误

`ScheduleBuildError` variant：

- `DuplicateStage(&'static str)`
- `UnknownStage(&'static str)`
- `UnknownStageAnchor(&'static str)`
- `InvalidFixedStep(String)`
- `ConflictingFixedStep(&'static str)`
- `InvalidParallelWaveMinimum(usize)`

`ScheduleError` 当前为：

```rust
MissingResource { system: String, resource: &'static str }
```

## 相关 API

- [系统](systems.md)
- [资源](resources.md)
- [延迟命令](commands.md)
- [插件](plugins-types.md)

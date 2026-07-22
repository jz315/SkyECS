# Scheduling, stages, and time

[API index](../../API.md) · [中文](../zh-CN/scheduling.md) · [Rustdoc](https://docs.rs/sky_ecs)

Modules: `sky_ecs`, `sky_ecs::stage`

## Stage labels

```rust
pub trait StageLabel: Send + Sync + 'static {
    fn name() -> &'static str { /* default: type_name::<Self>() */ }
}

pub struct First;
pub struct FixedUpdate;
pub struct PreUpdate;
pub struct Update;
pub struct PostUpdate;
pub struct Last;
```

Built-in stage order is `First`, `FixedUpdate`, `PreUpdate`, `Update`, `PostUpdate`, `Last`.
The built-in labels are copyable zero-sized values. Custom unit structs can use
`#[derive(StageLabel)]`.

## World schedule members

| Declaration | Effect |
|---|---|
| `pub fn stage<L: StageLabel>(&mut self, label: L) -> StageBuilder<'_>` | Builder for an installed stage; panics when missing. |
| `pub fn try_stage<L: StageLabel>(&mut self, label: L) -> Result<StageBuilder<'_>, ScheduleBuildError>` | Fallible installed-stage lookup. |
| `pub fn insert_stage_after<Anchor, L>(&mut self, anchor: Anchor, label: L) -> Result<StageBuilder<'_>, ScheduleBuildError>` | Installs a custom typed stage after an existing anchor. |
| `pub fn schedule_diagnostics(&mut self) -> ScheduleDiagnostics` | Compiles dirty waves if needed and returns an owned snapshot without running systems. |
| `pub fn tick(&mut self) -> Result<TickReport, ScheduleError>` | Advances with measured wall-clock delta; first delta is zero. |
| `pub fn tick_with_delta(&mut self, delta: f32) -> Result<TickReport, ScheduleError>` | Uses one value as frame and raw delta. |
| `pub fn tick_with_frame_delta(&mut self, frame_delta: f32, raw_delta: f32) -> Result<TickReport, ScheduleError>` | Separates clamped/scaled simulation input from raw elapsed input. |
| `pub fn shutdown(&mut self)` | Tears systems down in reverse order. |

Ticking or modifying/inspecting the schedule recursively panics. Ticking a poisoned World
panics. Missing resources found by preflight return an error without advancing time or running
systems.

Both explicit delta inputs are normalized to finite, non-negative values; negative, NaN, and
infinite values become zero. The scaled frame delta is normalized again after multiplication by
`Time::time_scale`.

## `StageBuilder`

```rust
pub struct StageBuilder<'a> { /* private fields */ }
```

| Declaration | Effect |
|---|---|
| `fixed(&mut self, step: FixedStep) -> Result<&mut Self, ScheduleBuildError>` | Configures fixed-step execution. |
| `parallel_wave_min_systems(&mut self, minimum: usize) -> Result<&mut Self, ScheduleBuildError>` | Minimum compatible wave width dispatched to Rayon; default 3, minimum 2, `usize::MAX` forces serial dispatch. |
| `add<Func, Marker>(&mut self, system: Func) -> &mut Self` | Adds an ordinary typed system using its Rust type name. |
| `add_named<Func, Marker>(&mut self, name: impl Into<String>, system: Func) -> &mut Self` | Adds a named ordinary system. |
| `add_exclusive<S: ExclusiveSystem>(&mut self, system: S) -> &mut Self` | Adds a serial full-World barrier. |
| `add_exclusive_named<S: ExclusiveSystem>(&mut self, name: impl Into<String>, system: S) -> &mut Self` | Named exclusive counterpart. |

Empty explicit system names panic. Compatible ordinary systems are grouped into waves; conflicts
and exclusive systems delimit parallel execution. Deferred command buffers flush at scheduler
boundaries in registration order.

## Fixed step

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedOverflow { Drop, Carry }

#[derive(Clone, Copy, Debug)]
pub struct FixedStep { /* private fields */ }
```

| Declaration | Contract |
|---|---|
| `FixedStep::hz(hz: u32) -> Self` | Panics when `hz == 0`. |
| `FixedStep::try_hz(hz: u32) -> Result<Self, ScheduleBuildError>` | Validated reciprocal frequency. |
| `FixedStep::seconds(seconds: f64) -> Result<Self, ScheduleBuildError>` | Requires finite `seconds > 0`. |
| `max_substeps(self, max_substeps: u32) -> Self` | Sets the per-frame budget; zero panics. |
| `overflow(self, overflow: FixedOverflow) -> Self` | Selects backlog dropping or carrying. |
| `step_seconds(self) -> f64` | Configured step duration. |
| `substep_limit(self) -> u32` | Configured budget. |

Constructors default to eight substeps and `FixedOverflow::Drop`. `FixedUpdate` initially uses
60 Hz. Its first explicit setting replaces that default; later equivalent settings are
idempotent and conflicting settings return `ConflictingFixedStep`.

## `Time`

```rust
#[derive(Debug, Clone)]
pub struct Time {
    pub delta: f32,
    pub frame_delta: f32,
    pub raw_delta: f32,
    pub elapsed: f64,
    pub raw_elapsed: f64,
    pub frame_count: u64,
    pub fixed_alpha: f32,
    pub time_scale: f32,
}
```

Getter methods `delta()`, `frame_delta()`, `raw_delta()`, `elapsed()`,
`raw_elapsed()`, and `fixed_alpha()` return their corresponding fields.

- `delta` is the current stage delta: fixed step inside a fixed stage, otherwise
  `frame_delta`.
- `frame_delta` and `elapsed` are affected by `time_scale`.
- `raw_delta` and `raw_elapsed` are not.
- The elapsed totals use `f64` so small frame deltas continue accumulating
  accurately during long-running sessions.
- `fixed_alpha` is the accumulated fraction for the built-in `FixedUpdate` stage.
- `Time::default()` initializes all counters/deltas to zero and `time_scale` to 1.

Every World owns permanent `Time` state both as `world.time` and as the built-in resource.
`Time` is `Sync` but intentionally not `Send`; ordinary systems receive shared `Res<Time>`.

## Reports and diagnostics

`TickReport` fields:

| Field | Type |
|---|---|
| `frame` | `u64` |
| `systems_run`, `waves_run`, `parallel_waves_run`, `sequential_waves_run`, `fixed_substeps` | `u32` |
| `dropped_fixed_time` | `f64` |

Schedule diagnostic declarations:

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

`CommandDiagnostics` exposes `last_enqueued`, `last_applied`, `last_discarded` as `usize`
and `total_enqueued`, `total_applied`, `total_discarded` as `u64`.

`SystemAccessDiagnostics` exposes component/resource read/write name vectors and
`uses_commands: bool`. Its
`conflict_reason(&self, other: &Self) -> Option<String>` returns the first typed component or
resource conflict.

## Errors

`ScheduleBuildError` variants:

- `DuplicateStage(&'static str)`
- `UnknownStage(&'static str)`
- `UnknownStageAnchor(&'static str)`
- `InvalidFixedStep(String)`
- `ConflictingFixedStep(&'static str)`
- `InvalidParallelWaveMinimum(usize)`

`ScheduleError` currently has:

```rust
MissingResource { system: String, resource: &'static str }
```

## See also

- [Systems](systems.md)
- [Resources](resources.md)
- [Commands](commands.md)
- [Plugins](plugins-types.md)

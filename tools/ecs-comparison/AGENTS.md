# ECS Comparison Harness Instructions

## Scope

These instructions apply to `tools/ecs-comparison/`. This harness has a newer
Rust requirement than the published Sky crates; do not propagate that MSRV to
the rest of the workspace.

Keep this file limited to durable benchmark rules. Task plans, temporary
experiments, raw local results, and publication progress belong outside
`AGENTS.md`.

## Fairness and Workload Contracts

- Every adapter must execute the same entity count, component set, initial
  values, mutation kernel, and completion semantics for a workload.
- Place fixture generation and engine-native input construction outside the
  timer unless conversion is explicitly part of the workload contract.
- Include destination allocation, metadata updates, and required iterator
  completion when the timed API performs them.
- Use distinct row and column values in contract tests even when the benchmark
  fixture intentionally repeats values.
- Do not use private or unsupported engine internals to manufacture a faster
  adapter path.
- Reuse adapter helpers from benchmarks and contract validation where practical
  so their semantics cannot drift independently.

## Fastest Public API Requirement

Every timed adapter phase must use that engine's fastest supported public API
for the exact operation and workload semantics. "Idiomatic", "simple", or
"already used elsewhere" is not evidence that an API is the fastest.

- Enumerate all plausible public API candidates before adding or materially
  changing a workload. Include cached/prepared queries, component views or
  accessors, native bulk operations, and batch structural operations when the
  engine exposes them.
- Compare candidates with the same fixture, mutation kernel, checksum, setup
  boundary, compiler flags, and completion semantics. Run four
  alternating AB/BA order rotations on the publication toolchain and target.
- Select the fastest repeatable candidate. Retain the candidate benchmark or a
  reproducible certification command and record the winner in the ledger below.
  When candidates are statistically indistinguishable, record the tie and use
  the lower-median supported path.
- A candidate may be rejected only because it changes the workload contract,
  uses private or unsupported internals, omits required completion work, or
  relies on caching/precomputation that the contract does not permit. Record the
  rejection reason next to the certification evidence.
- Apply the same caching and setup policy to every engine. For example, cached
  entity/component references are allowed only when the common contract makes
  the referenced identity stable and permits equivalent setup outside timing.
- Re-certify affected rows after an engine version change, workload-contract
  change, target/toolchain change, or discovery of another plausible API.
- Update this ledger in the same change as an adapter path. A row marked
  `uncertified` or an operation missing from the ledger blocks publication of
  that workload.

The entries below are operation-specific selections, not claims that one API is
universally fastest for an engine. They describe the required paths in the
current harness; the evidence state determines whether a result may be
published.

| Adapter | Dense/prepared iteration | Entity/random access | Native bulk construction |
|---|---|---|---|
| Sky | `PreparedQuery::for_each_chunk_fn` for the simple dense kernel; gameplay retains the closure form because the provisional function winner did not clear the full-frame gate | `EntityAccessor<T>::get` for comparable EntityId access; `PreparedEntityAccess<T>::iter` for the fixed-sequence scenario; `PreparedEntityView<Q>::get/get_mut` for arbitrary multi-component items | `World::spawn_columns` with prepared component columns |
| hecs | Provisional: 10K/100K use `World::query_mut().into_iter_batched(u32::MAX)`; 1M uses prepared matching `Archetype::get` columns. Publication remains uncertified until the candidate bench is repeated on the publication target | `PreparedQuery::view_mut(...).get` / `get_mut` | `World::spawn_column_batch` with a completed `ColumnBatch` |
| Flecs C | prepared `ecs_query_t` with `ecs_query_iter` / `ecs_query_next` and direct `ecs_field` columns | `ecs_ref_init_id` in permitted stable-identity setup plus `ecs_ref_get_id`; otherwise `ecs_get_id` / `ecs_get_mut_id`. Gameplay must use the latter because it reads `TargetSlot` and builds the target list each frame | `ecs_bulk_init` with sorted component IDs and prepared columns |
| Bevy ECS | reusable `QueryState::iter_mut` | reusable `QueryState::get_manual` / `get_mut` | `World::spawn_batch` with prepared bundles |
| Shipyard | borrowed `ViewMut`/`View` tuple with `IntoIter::iter` | borrowed `View<T>` / `ViewMut<T>` with `Get::get` | `World::bulk_add_entity` with prepared bundles |
| FreeCS | warmed `World::for_each_mut(mask, ...)` | generated typed component getters such as `get_position` / `get_cooldown_mut` | `World::spawn_batch` with prepared component columns |

| Adapter | Gameplay component changes | Gameplay entity recycle |
|---|---|---|
| Sky | `World::insert` / `World::remove` | `World::despawn` then tuple `World::spawn` |
| hecs | `World::insert_one` / `World::remove_one` | `World::despawn` then tuple `World::spawn` |
| Flecs C | `ecs_add_id` / `ecs_remove_id` | `ecs_delete` then `ecs_new_w_table` with direct component initialization |
| Bevy ECS | `EntityWorldMut::insert` / `remove` | `World::despawn` then tuple `World::spawn` |
| Shipyard | `World::add_component` / `delete_component` | `World::delete_entity` then `World::add_entity` |
| FreeCS | generated typed setters/removers | `World::despawn_entities` batch then `World::spawn_batch` |

| Adapter | Gameplay AI source | Gameplay target Position |
|---|---|---|
| Sky | `PreparedEntityView<(&TargetSlot, &mut Cooldown)>`; canonical winner on `b157e347` | `EntityAccessor<Position>::get`; canonical winner on `b157e347` |
| hecs | `PreparedQuery<(&TargetSlot, &mut Cooldown)>::view_mut().get_mut` (uncertified) | `PreparedQuery<&Position>::view_mut().get` (uncertified) |
| Flecs C | per-frame `ecs_get_id(TargetSlot)` plus `ecs_get_mut_id(Cooldown)` (uncertified) | `ecs_get_id(Position)` over the generated target list (uncertified) |
| Bevy ECS | reusable tuple `QueryState::get_mut` (uncertified) | reusable `QueryState::get_manual` (uncertified) |
| Shipyard | borrowed `View<TargetSlot>` / `ViewMut<Cooldown>` tuple and `Get::get` (uncertified) | borrowed `View<Position>` and `Get::get` (uncertified) |
| FreeCS | generated `get_target_slot` / `get_cooldown_mut` (uncertified) | generated `get_position` (uncertified) |

The native-bulk scenario has contract tests and explicit native prepared inputs.
Sky API experiments live in `crates/sky_ecs/benches`; the canonical comparison
contains only the selected paths and never chooses an API on a shared runner.
Re-run `SKY_ECS_CERTIFY_GAMEPLAY_API=1 cargo bench -p sky_ecs_comparison --bench
api_candidates --features api-experiments -- sky` on a clean publication target after
relevant workload, toolchain, or storage changes. This command records raw
AB/BA rounds and the full-frame gate; the ordinary command runs Criterion's
candidate groups.
The `b157e347` Windows x86-64 certificate retained the production combination
`Closure | PreparedEntityView | EntityAccessor`: the function iteration path
was only a provisional winner and did not clear the 2% full-frame gate. Raw
rounds are stored in
`benches/certifications/sky-gameplay-api.windows-x86_64.b157e347.json`.
All other gameplay rows remain `uncertified` until phase-specific
candidate comparisons cover every plausible API; therefore new gameplay-frame
numbers are diagnostic and must not be published yet.

## Dense Prepared Iteration

`cargo bench -p sky_ecs_comparison --bench api_candidates --features
api-experiments -- hecs_dense` screens shared and unique prepared queries, world-cached
unique queries, prepared views, serial batched iteration, and safe public
archetype-column access. Finalists must then be rerun locally in alternating
AB/BA order for 10K, 100K, and 1M. The 1M column plan caches only matching `&Archetype`
references; component guards are acquired and released inside each timed
traversal, and Rust prevents structural World mutation while that plan is alive.
Local selection runs are diagnostic only. Re-run the candidate bench on the
publication toolchain and target before clearing the hecs dense row's
uncertified status or publishing comparison numbers that use these paths.

The canonical Sky adapter for the 10K, 100K, and 1M simple prepared-iteration
workloads uses `PreparedQuery::for_each_chunk_fn` and a reusable non-capturing
function:

```rust
#[inline(never)]
fn move_chunk(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}
```

Do not replace this boundary with a tuple-valued capturing closure without new
assembly and benchmark evidence. The plain function preserves independent
slice alias contracts after raw ECS columns have been reconstructed.

Use `crates/sky_ecs/benches/chunk_cost.rs` to distinguish flat computation,
segmentation, descriptor walking, prepared dispatch, and full ECS computation.
Do not attribute a dense difference to matching or dispatch when those costs
are only tens of nanoseconds relative to the component kernel.

Record actual chunk lengths for storage experiments. The current known-batch
policy may use unpooled oversized chunks, while repeated smaller batches can
retain the normal 4 MiB layout; keep both cases as diagnostics.

## EntityId and Fixed-Sequence Access

`entity_id_random_access` is comparable only when every adapter begins each
timed lookup from its Entity ID. Sky therefore uses `EntityAccessor::get` and
Flecs uses `ecs_get_id`; no adapter may substitute a prepared address plan.

`scenario_fixed_sequence_access` separately measures plan build, steady
traversal, and build amortized over 1/4/16/64 traversals. Plans may retain direct
component references only while the fixture World remains structurally frozen.
Plan payload bytes exclude allocator bookkeeping. Sky's crate-local candidates
are reproduced with `cargo bench -p sky_ecs --bench random_access`.

## Native Bulk Construction

`scenario_native_bulk_construction/insert_10k` measures each engine's fastest
public native bulk capability from an empty schema-prepared world and a fully
prepared engine-native input batch. It is a scenario rather than a comparable
neutral-input workload.

| Adapter | Public path | Prepared input |
|---|---|---|
| Sky | `World::spawn_columns` | four component `Vec`s |
| hecs | `World::spawn_column_batch` | completed `hecs::ColumnBatch` |
| Flecs C | `ecs_bulk_init` | sorted IDs and four C++ vectors |
| Bevy ECS | `World::spawn_batch` | `Vec<SuiteBundle>` |
| Shipyard | `World::bulk_add_entity` | `Vec<SuiteBundle>` |
| FreeCS | `World::spawn_batch` | four consumed component columns |

Destroy the benchmark context after timing. Explicitly exhaust or drop returned
iterators inside the measured closure when that action completes insertion.
Keep `single_insert_10k` as the repeated single-spawn comparison; it is a
different workload.

## Adapter and Native-Code Boundaries

- Keep adapter modules symmetric and place shared fixtures/catalog data in
  `src/common/`.
- Validate Flecs component-ID ordering and data-pointer pairing whenever its
  native column mapping changes.
- Keep C/C++ build logic in `build_support/` and native sources in `native/`.
- Do not silently change compiler flags for one engine in a published run.
- Gameplay traces must match the common reference trace before their timings
  are considered comparable.

## Results and Publication

- Historical results stay attached to their original commit, workload name,
  toolchain, target CPU policy, and runner.
- Never relabel old results after an adapter or workload contract changes.
- Do not combine generic x86-64, `target-cpu=native`, GitHub-hosted, and local
  measurements in one comparison table.
- Quick single-order runs are diagnostic only. Four cyclic rotations are the
  bounded protocol but do not form a complete position-bias block, so the
  report must show order bias as N/A. Six rotations cover every engine position
  once; all reports retain exact orders and per-run distributions.
- Update English and Chinese benchmark documents from a completed publication
  run, not by hand from an isolated Criterion sample.

## Verification

```text
cargo fmt --all -- --check
cargo check -p sky_ecs_comparison --all-targets
cargo test -p sky_ecs_comparison --test contracts
cargo compare-ecs -- <workload>/<adapter> --exact
cargo bench -p sky_ecs_comparison --bench gameplay_phases -- --noplot
```

For dense storage or query changes, also run the `chunk_cost` diagnostic and
inspect optimized assembly on the publication target. Hardware-counter claims
should compare cycles, instructions, cache misses, and DTLB misses on a
controlled Linux machine.

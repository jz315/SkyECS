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

## Official Benchmark Taxonomy

The GitHub Compare-ECS benchmark has four publication sections in this fixed
order. Do not move a workload between sections merely to simplify the report.

1. **Comparable** contains exactly these operation families:
   - Entity construction: repeated single-entity construction at 10K and bulk
     construction from four neutral component columns at 10K.
   - Entity operations: spawn/despawn 1K and add/remove component 1K.
   - Sky-authored EntityId random access: hot 10K and warm 100K.
   - Prepared iteration: 10K, 100K, and 1M.
   - Fragmented iteration: 26 × 400.
2. **Random Fragmentation** is a separate official section containing the full
   tag and data-component matrix: 6 and 8 shapes at 1/4 terms, plus 10 and 16
   shapes at 1/4/8 terms. Keep the section title neutral; acknowledge the
   benchmark's external origin in prose, never by adding "ported", "移植", or
   similar wording to the benchmark name.
3. **Gameplay Scenario** contains only the canonical gameplay workload:
   full frame, iteration, AI source lookup, target Position lookup, status
   transition, and projectile recycle. Bulk construction and
   fixed-sequence access are not scenarios.
4. **Diagnostic** contains heavy compute and any future explicitly diagnostic
   probes. Diagnostics do not participate in comparative wins.

The formal GitHub report therefore has 37 rows: 10 Comparable, 20 Random
Fragmentation, 6 Gameplay Scenario, and 1 Diagnostic. Treat these counts as a
publication contract and update this file deliberately if the official suite
changes.

Fixed-sequence entity access is a local API-strategy experiment. It measures
plan build, steady traversal, and build amortized over 1/4/16/64 traversals,
but it must not be registered in the formal `comparison` bench, publisher
catalog, GitHub performance workflow, or published result tables. Keep it in a
feature-gated local candidate bench; CI may compile it but must not execute it
or use it to select a production API.

Gameplay phase rows are formal GitHub results, not local API candidates. Every
phase benchmark must advance the same complete evolving state machine as the
full frame while timing only the selected phase. Do not replace these rows with
isolated worlds or omit them from the publication artifact.

## Fastest Public API Requirement

Every timed adapter phase must use that engine's fastest supported public API
for the exact operation and workload semantics. "Idiomatic", "simple", or
"already used elsewhere" is not evidence that an API is the fastest.

- Enumerate all plausible public API candidates before adding or materially
  changing a workload. Include cached/prepared queries, component views or
  accessors, bulk-construction paths, and batch structural operations when the
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

| Adapter | Dense/prepared iteration | Entity/random access | Bulk construction from columns |
|---|---|---|---|
| Sky | Unified `PreparedQuery::for_each_chunk`: the simple dense kernel passes a reusable non-capturing function, while gameplay and the register-heavy diagnostic kernel pass an inlineable capturing closure | `EntityAccessor<T>::get` for comparable EntityId access; `PreparedEntityAccess<T>::iter` for the local fixed-sequence experiment; `PreparedEntityAccessor<T>::get` for reusable single-component items; `PreparedEntityView<Q>::get/get_mut` for arbitrary multi-component items | `World::spawn_columns` with prepared component columns |
| hecs | Provisional: 10K/100K use `World::query_mut().into_iter_batched(u32::MAX)`; 1M uses prepared matching `Archetype::get` columns. Publication remains uncertified until the candidate bench is repeated on the publication target | `PreparedQuery::view_mut(...).get` / `get_mut` | prepare `ColumnBatchType` with the static schema in setup; `into_batch`, writers, build and `World::spawn_column_batch` remain timed |
| Flecs C | prepared `ecs_query_t` with `ecs_query_iter` / `ecs_query_next` and direct `ecs_field` columns | `ecs_ref_init_id` in permitted stable-identity setup plus `ecs_ref_get_id`; otherwise `ecs_get_id` / `ecs_get_mut_id`. Gameplay must use the latter because it reads `TargetSlot` and builds the target list each frame | prepare the stable component-ID ordering and empty target table in setup; build the per-batch descriptor in timing, then call `ecs_bulk_init` with that table |
| Bevy ECS | reusable `QueryState::iter_mut` | reusable `QueryState::get_manual` / `get_mut` | drain neutral columns into `World::spawn_batch` |
| Shipyard | borrowed `ViewMut`/`View` tuple with `IntoIter::iter` | borrowed `View<T>` / `ViewMut<T>` with `Get::get` | drain neutral columns into `World::bulk_add_entity` |
| FreeCS | warmed `World::for_each_mut(mask, ...)` | generated typed component getters such as `get_position` / `get_cooldown_mut` | drain neutral columns inside `World::spawn_batch` |

| Adapter | Gameplay component changes | Gameplay entity recycle |
|---|---|---|
| Sky | `World::insert` / `World::remove` | `World::despawn` then tuple `World::spawn` |
| hecs | `World::insert_one` / `World::remove_one` | `World::despawn` then tuple `World::spawn` |
| Flecs C | `ecs_add_id` / `ecs_remove_id` | `ecs_delete` then `ecs_new_w_table` with direct component initialization |
| Bevy ECS | `EntityWorldMut::insert` / `remove` | `World::despawn` then tuple `World::spawn` |
| Shipyard | `World::add_component` / `delete_component` | `World::delete_entity` then `World::add_entity` |
| FreeCS | generated typed setters/removers | `World::despawn_entities` batch then `World::spawn_batch` |

Formal `entity_ops` uses these operation-specific paths:

| Adapter | Spawn/despawn 1K | Add/remove valued component 1K |
|---|---|---|
| Sky | tuple `World::spawn` / `World::despawn` | `World::insert` / `World::remove` |
| hecs | tuple `World::spawn` / `World::despawn` | `World::insert_one` / `World::remove_one` |
| Flecs C | `ecs_new_w_table`, direct `ecs_get_mut_id` initialization, then `ecs_delete` | `ecs_emplace_id`, direct initialization, then `ecs_remove_id` |
| Bevy ECS | tuple `World::spawn` / `World::despawn` | `EntityWorldMut::insert` / `remove` |
| Shipyard | `World::add_entity` / `delete_entity` | `World::add_component` / `delete_component` |
| FreeCS | generated one-row `spawn_batch` / `despawn_entities` | generated typed setter / remover |

The feature-gated `api_candidates_structural_writes` suite compares hecs bulk
schema placement, Flecs target-table and column-mapping preparation, three
Flecs single-entity construction paths, and three Flecs valued add/remove
paths with identical state contracts.
Run the clean-worktree certification with
`SKY_ECS_CERTIFY_STRUCTURAL_API=1 cargo bench -p sky_ecs_comparison --bench
api_candidates --features api-experiments -- structural`. A dirty diagnostic
uses the same AB/BA measurements but is explicitly ineligible for production
evidence. Until the clean certificate is recorded, these structural selections
remain provisional and block publication.

| Adapter | Gameplay AI source | Gameplay target Position |
|---|---|---|
| Sky | `PreparedEntityView<(&TargetSlot, &mut Cooldown)>`; canonical winner confirmed on `45128af` | `PreparedEntityAccessor<Position>::get`; canonical winner confirmed on `45128af` |
| hecs | `PreparedQuery<(&TargetSlot, &mut Cooldown)>::view_mut().get_mut` (uncertified) | `PreparedQuery<&Position>::view_mut().get` (uncertified) |
| Flecs C | per-frame `ecs_get_id(TargetSlot)` plus `ecs_get_mut_id(Cooldown)` (uncertified) | `ecs_get_id(Position)` over the generated target list (uncertified) |
| Bevy ECS | reusable tuple `QueryState::get_mut` (uncertified) | reusable `QueryState::get_manual` (uncertified) |
| Shipyard | borrowed `View<TargetSlot>` / `ViewMut<Cooldown>` tuple and `Get::get` (uncertified) | borrowed `View<Position>` and `Get::get` (uncertified) |
| FreeCS | generated `get_target_slot` / `get_cooldown_mut` (uncertified) | generated `get_position` (uncertified) |

The bulk-construction workload has contract tests and starts every adapter from
the same four neutral component columns.
Sky library API experiments live in `crates/sky_ecs/benches`. An exact
adapter-codegen experiment that depends on comparison-owned workload types may
instead live in the feature-gated `api_candidates` bench; the Heavy Compute
function-boundary/inline-closure candidates are reproduced with `cargo bench
-p sky_ecs_comparison --bench api_candidates --features api-experiments --
sky_heavy_compute_api`. The canonical comparison contains only the selected
path and never chooses an API on a shared runner.
Re-run `SKY_ECS_CERTIFY_GAMEPLAY_API=1 cargo bench -p sky_ecs_comparison --bench
api_candidates --features api-experiments -- sky` on a clean publication target after
relevant workload, toolchain, or storage changes. This command records raw
AB/BA rounds and the full-frame gate; the ordinary command runs Criterion's
candidate groups.
The `45128af` Windows x86-64 recertification retained
`Closure | PreparedEntityView | PreparedEntityAccessor`. The tuple view and
reusable single-component accessor remained the AI and Position Condorcet
winners. The iteration function won only the order-neutral median fallback,
and its proposed full frame did not clear the 2% gate, so the production
closure remained selected. Raw rounds are stored in
`benches/certifications/sky-gameplay-api.windows-x86_64.45128af.json`.
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
workloads uses `PreparedQuery::for_each_chunk` with a reusable non-capturing
function:

```rust
#[inline(never)]
fn move_chunk(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}
```

Do not replace this boundary with a capturing closure without new assembly and
benchmark evidence. The unified API still passes each component slice as an
independent function parameter, preserving its alias contract after raw ECS
columns have been reconstructed.

Use `crates/sky_ecs/benches/chunk_cost.rs` to distinguish flat computation,
segmentation, descriptor walking, prepared dispatch, and full ECS computation.
Do not attribute a dense difference to matching or dispatch when those costs
are only tens of nanoseconds relative to the component kernel.

Record actual chunk lengths for storage experiments. The current known-batch
policy may use unpooled oversized chunks, while repeated smaller batches can
retain the normal 4 MiB layout; keep both cases as diagnostics.

## EntityId and Local Fixed-Sequence Access

`entity_id_random_access` is comparable only when every adapter begins each
timed lookup from its Entity ID. Sky therefore uses `EntityAccessor::get` and
Flecs uses `ecs_get_id`; no adapter may substitute a prepared address plan.
This Sky-authored workload stays in the Comparable section and must not be
merged with the separately presented Random Fragmentation matrix.

The local fixed-sequence experiment separately measures plan build, steady
traversal, and build amortized over 1/4/16/64 traversals. Plans may retain
direct component references only while the fixture World remains structurally
frozen. Plan payload bytes exclude allocator bookkeeping. Do not use a
`scenario_` prefix for this experiment. Sky's crate-local candidates are
reproduced with `cargo bench -p sky_ecs --bench random_access`.

## Entity Construction

Repeated single-entity construction at 10K and bulk construction at 10K belong
to the same Comparable Entity Construction family. Present them next to each
other in one table. Bulk construction starts from an empty schema-prepared
World and the same four neutral component `Vec`s for every adapter.

Source-value generation and source-`Vec` allocation are setup work. The timed
region must include every per-batch engine-native allocation, conversion,
fill/finalize operation, insertion call, and required iterator/commit work.
Consume source columns with `drain(..)` so their allocation is released with
the benchmark context outside timing. A completed `hecs::ColumnBatch`, prepared
bundle vector, or equivalent final engine storage must never be built before
entering the timed region.

| Adapter | Timed public path | Input at timing boundary |
|---|---|---|
| Sky | `World::spawn_columns` | four component `Vec`s |
| hecs | prepared `ColumnBatchType` + timed `into_batch`, writers, `build` and `World::spawn_column_batch` | four component `Vec`s plus the static batch schema |
| Flecs C | prepared component-ID ordering and empty target table + timed descriptor construction and `ecs_bulk_init` | four C++ component vectors plus the static ID mapping and table |
| Bevy ECS | drain zipped bundles into `World::spawn_batch` | four component `Vec`s |
| Shipyard | drain zipped bundles into `World::bulk_add_entity` | four component `Vec`s |
| FreeCS | `World::spawn_batch` with timed column writes | four component `Vec`s |

Destroy the benchmark context after timing. Explicitly exhaust or drop returned
iterators inside the measured closure when that action completes insertion.
Use Criterion `LargeInput` batching for both 10K construction rows. Each
measured input becomes a large World, so `SmallInput` retains enough completed
Worlds in one timed batch to turn allocator retention into part of the result.
This batching rule does not move native batch construction or any other engine
work out of the timed closure.
Keep `single_insert_10k` as the repeated single-spawn row and display it as
"Individual construction 10K" or "逐实体构建 10K" so readers do not mistake
the name for a single-entity benchmark. Use
`entity_construction/bulk_from_columns_10k` for the bulk row. Do not retain,
publish, or move the former prebuilt-native-batch commit benchmark into a local
or Diagnostic suite. Do not add a separate derived speedup table for individual
versus bulk construction.

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
- The human-facing GitHub report starts with one source line linking to the
  GitHub Actions run, then immediately presents the four official sections.
  Do not add a run-metadata table. Commit, contracts, runner, toolchain, and raw
  distributions remain available through the Actions run and its artifact.
- The root README keeps a compact snapshot from the latest completed formal
  run: all ten Comparable rows plus the Gameplay full-frame row, in that order.
  Do not add Random Fragmentation, Gameplay phase, Heavy Compute, or
  Fixed-Sequence rows to the README. Link to the complete benchmark document
  instead.
- The English and Chinese benchmark documents record the complete 37-row
  formal report from the same run: 10 Comparable, 20 Random Fragmentation,
  6 Gameplay Scenario, and 1 Diagnostic. Keep their values, bold winners,
  noise marks, and N/A cells synchronized.
- Use the engine column order `Sky`, `hecs`, `Bevy`, `Flecs C`, `FreeCS`,
  `Shipyard` in every table. Lower is faster; bold only the lowest median in a
  row, mark noisy cells with `†`, and render unsupported or unfinished cells as
  `N/A`.
- The Comparable table uses `Test` and `Scale/Mode` columns so its ten rows can
  group both Entity Construction modes without creating an auxiliary table.
  Random Fragmentation uses separate Tag and Data Component tables. Gameplay
  uses one table with full frame first and its five phases below it.
- Do not commit CI summary JSON, runner dumps, hash manifests, or copied
  publication artifacts solely to render the benchmark documentation. Keep
  the human-facing tables concise and obtain provenance from the linked
  Actions artifact.

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

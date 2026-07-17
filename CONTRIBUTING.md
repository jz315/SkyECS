# Contributing

Contributions are welcome. Keep changes focused and include tests for public
behavior or storage invariants.

Before opening a pull request, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo bench -p sky_ecs --no-run
cargo bench -p sky_ecs_comparison --bench comparison --no-run
```

Typed query, archetype, chunk, and structural transition paths are
performance-sensitive. Changes there should include benchmark evidence and must
preserve generational entity validity, moved-entity locations, resource
lifetimes, and non-`Copy` drop behavior.

Cross-engine claims belong only in `tools/ecs-comparison`. Those workloads must
use safe public APIs available in every compared ECS, with prepared/query state
created outside the timed loop.

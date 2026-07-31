#![allow(dead_code)]

use criterion::{criterion_group, criterion_main};

#[path = "structural_writes/bulk.rs"]
mod bulk;
#[path = "structural_writes/fixtures.rs"]
mod fixtures;
#[path = "structural_writes/migration.rs"]
mod migration;
#[path = "structural_writes/spawn.rs"]
mod spawn;

criterion_group!(
    benches,
    spawn::bench_spawn_and_despawn,
    bulk::bench_bulk_metadata,
    migration::bench_migration_copy_spans
);
criterion_main!(benches);

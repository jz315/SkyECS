use criterion::{criterion_group, criterion_main};

#[path = "gameplay_phases/suite.rs"]
mod suite;

criterion_group!(gameplay_phase_benches, suite::bench_gameplay_phases);
criterion_main!(gameplay_phase_benches);

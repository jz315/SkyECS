use criterion::Criterion;

pub fn run(criterion: &mut Criterion) {
    sky_ecs_comparison::sky::bench_heavy_compute_candidates(criterion);
}

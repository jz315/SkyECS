use super::heavy_compute::{
    heavy_world, process_heavy_chunk, process_heavy_entity, run_inline_chunk_closure,
};
use super::*;
use criterion::Criterion;
use std::time::Duration;

#[inline(never)]
fn process_heavy_chunk_legacy(
    total_checksum: &mut u64,
    positions: &mut [PositionComponent],
    transforms: &[TransformComponent],
) {
    let mut checksum = 0_u64;
    for (position, transform) in positions.iter_mut().zip(transforms) {
        let mut matrix = transform.0;
        for _ in 0..HEAVY_INVERT_COUNT {
            matrix = matrix.inverse();
        }
        position.0 = matrix.transform_vector(position.0);
        checksum = add_full_position_checksum(checksum, position);
    }
    *total_checksum = total_checksum.wrapping_add(checksum);
}

#[inline(never)]
fn process_heavy_chunk_boundary(
    total_checksum: &mut u64,
    positions: &mut [PositionComponent],
    transforms: &[TransformComponent],
) {
    *total_checksum = total_checksum.wrapping_add(process_heavy_chunk(positions, transforms));
}

fn run_legacy_function_boundary(
    world: &mut World,
    query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) -> u64 {
    let mut checksum = 0_u64;
    query.for_each_chunk(world, |positions, transforms| {
        process_heavy_chunk_legacy(&mut checksum, positions, transforms);
    });
    checksum
}

fn run_function_boundary(
    world: &mut World,
    query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) -> u64 {
    let mut checksum = 0_u64;
    query.for_each_chunk(world, |positions, transforms| {
        process_heavy_chunk_boundary(&mut checksum, positions, transforms);
    });
    checksum
}

fn run_entity_closure(
    world: &mut World,
    query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) -> u64 {
    let mut checksum = 0_u64;
    query.for_each(world, |position, transform| {
        checksum = checksum.wrapping_add(process_heavy_entity(position, transform));
    });
    checksum
}

pub fn bench_heavy_compute_candidates(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sky_heavy_compute_api");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30);

    group.bench_function("legacy_function_boundary", |bencher| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        bencher.iter(|| {
            black_box(run_legacy_function_boundary(&mut world, &mut query));
            black_box(&world);
        });
    });

    group.bench_function("function_boundary", |bencher| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        bencher.iter(|| {
            black_box(run_function_boundary(&mut world, &mut query));
            black_box(&world);
        });
    });

    group.bench_function("inline_chunk_closure", |bencher| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        bencher.iter(|| {
            black_box(run_inline_chunk_closure(&mut world, &mut query));
            black_box(&world);
        });
    });

    group.bench_function("entity_closure", |bencher| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        bencher.iter(|| {
            black_box(run_entity_closure(&mut world, &mut query));
            black_box(&world);
        });
    });

    group.finish();
}

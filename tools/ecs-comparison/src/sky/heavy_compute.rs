use super::*;

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

// Keep the long-running component loop at a real function boundary. The two
// slice references then retain their independent no-alias contracts through
// optimization, matching the native adapter's standalone heavy function. Do
// not turn this back into a capturing closure: the benchmark intentionally
// exercises PreparedQuery's alias-aware function path.
#[inline(never)]
fn process_heavy_chunk(
    total_checksum: &mut u64,
    positions: &mut [PositionComponent],
    transforms: &[TransformComponent],
) {
    // Reduce locally so the hot entity loop does not repeatedly access the
    // caller-owned context. Only one checksum value crosses the boundary per
    // chunk.
    let mut checksum = 0_u64;
    for (position, transform) in positions.iter_mut().zip(transforms) {
        let mut matrix = transform.0;
        // Each inverse depends on the preceding result. This prevents the
        // optimizer from replacing one hundred inversions with a cheaper
        // precomputed expression.
        for _ in 0..HEAVY_INVERT_COUNT {
            matrix = matrix.inverse();
        }
        position.0 = matrix.transform_vector(position.0);
        checksum = add_full_position_checksum(checksum, position);
    }
    *total_checksum = total_checksum.wrapping_add(checksum);
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/sky", |b| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        b.iter(|| {
            // Criterion iterations get independent accumulator state while the
            // prepared query and World remain outside the timed setup boundary.
            let mut checksum = 0_u64;
            query.for_each_chunk_fn_with(&mut world, &mut checksum, process_heavy_chunk);
            black_box(checksum);
            black_box(&world);
        });
    });
}

use super::*;

pub(super) fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

#[inline(always)]
pub(super) fn process_heavy_entity(
    position: &mut PositionComponent,
    transform: &TransformComponent,
) -> u64 {
    let mut matrix = transform.0;
    // Each inverse depends on the preceding result. This prevents the
    // optimizer from replacing one hundred inversions with a cheaper
    // precomputed expression.
    for _ in 0..HEAVY_INVERT_COUNT {
        matrix = matrix.inverse();
    }
    position.0 = matrix.transform_vector(position.0);
    add_full_position_checksum(0, position)
}

// Keep the arithmetic helper inlineable so LLVM can optimize the
// register-heavy matrix kernel together with the typed chunk traversal.
#[inline(always)]
pub(super) fn process_heavy_chunk(
    positions: &mut [PositionComponent],
    transforms: &[TransformComponent],
) -> u64 {
    // Reduce locally so the hot entity loop does not repeatedly access the
    // caller-owned context. Only one checksum value crosses the boundary per
    // chunk.
    let mut checksum = 0_u64;
    for (position, transform) in positions.iter_mut().zip(transforms) {
        checksum = checksum.wrapping_add(process_heavy_entity(position, transform));
    }
    checksum
}

#[inline(always)]
pub(super) fn run_inline_chunk_closure(
    world: &mut World,
    query: &mut PreparedQuery<(&mut PositionComponent, &TransformComponent)>,
) -> u64 {
    let mut checksum = 0_u64;
    query.for_each_chunk(world, |positions, transforms| {
        checksum = checksum.wrapping_add(process_heavy_chunk(positions, transforms));
    });
    checksum
}

#[cfg(test)]
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

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/sky", |b| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        b.iter(|| {
            // Criterion iterations get independent accumulator state while the
            // prepared query and World remain outside the timed setup boundary.
            let checksum = run_inline_chunk_closure(&mut world, &mut query);
            black_box(checksum);
            black_box(&world);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position_bits(world: &World) -> Vec<[u32; 3]> {
        let mut query = PreparedQuery::<&PositionComponent>::new();
        let mut positions = Vec::with_capacity(HEAVY_ENTITY_COUNT);
        query.for_each(world, |position| {
            positions.push([
                position.0.x.to_bits(),
                position.0.y.to_bits(),
                position.0.z.to_bits(),
            ]);
        });
        positions
    }

    #[test]
    fn inline_chunk_and_entity_candidates_have_identical_semantics() {
        let mut chunk_world = heavy_world();
        let mut entity_world = heavy_world();
        let mut chunk_query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        let mut entity_query =
            PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();

        let chunk_checksum = run_inline_chunk_closure(&mut chunk_world, &mut chunk_query);
        let entity_checksum = run_entity_closure(&mut entity_world, &mut entity_query);

        assert_eq!(chunk_checksum, entity_checksum);
        assert_eq!(position_bits(&chunk_world), position_bits(&entity_world));
    }
}

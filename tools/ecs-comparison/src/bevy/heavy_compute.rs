use super::*;

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}
pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/bevy", |b| {
        let mut world = heavy_world();
        let mut query = world.query::<(&mut PositionComponent, &TransformComponent)>();
        b.iter(|| {
            let mut checksum = 0_u64;
            for (mut position, transform) in query.iter_mut(&mut world) {
                let mut matrix = transform.0;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = matrix.inverse();
                }
                position.0 = matrix.transform_vector(position.0);
                checksum = add_full_position_checksum(checksum, &position);
            }
            black_box(checksum);
            black_box(&world);
        });
    });
}

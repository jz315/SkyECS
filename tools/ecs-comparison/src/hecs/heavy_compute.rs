use super::dense_iteration::assert_prepared_count;
use super::*;

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}
pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/hecs", |b| {
        let world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::default();
        assert_prepared_count(&mut query, &world, HEAVY_ENTITY_COUNT);
        b.iter(|| {
            let mut checksum = 0_u64;
            for (position, transform) in query.query(&world).iter() {
                let mut matrix = transform.0;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = matrix.inverse();
                }
                position.0 = matrix.transform_vector(position.0);
                checksum = add_full_position_checksum(checksum, position);
            }
            black_box(checksum);
            black_box(&world);
        });
    });
}

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
            for (position, transform) in query.query(&world).iter() {
                let mut matrix = transform.0;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = matrix
                        .invert()
                        .expect("heavy matrix should remain invertible");
                }
                position.0 = matrix.transform_vector(position.0);
            }
            black_box(&world);
        });
    });
}

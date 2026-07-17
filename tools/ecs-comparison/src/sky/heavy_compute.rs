use super::*;

fn heavy_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/sky", |b| {
        let mut world = heavy_world();
        let mut query = PreparedQuery::<(&mut PositionComponent, &TransformComponent)>::new();
        assert_eq!(query.count(&world), HEAVY_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk(&mut world, |(positions, transforms)| {
                for (position, transform) in positions.iter_mut().zip(transforms) {
                    let mut matrix = transform.0;
                    for _ in 0..HEAVY_INVERT_COUNT {
                        matrix = matrix
                            .invert()
                            .expect("heavy matrix should remain invertible");
                    }
                    position.0 = matrix.transform_vector(position.0);
                }
            });
            black_box(&world);
        });
    });
}

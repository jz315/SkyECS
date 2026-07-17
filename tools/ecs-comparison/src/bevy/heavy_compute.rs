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
            for (mut position, transform) in query.iter_mut(&mut world) {
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

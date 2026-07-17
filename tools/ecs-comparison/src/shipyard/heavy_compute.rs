use super::*;

fn heavy_world() -> World {
    let mut world = World::new();
    world.bulk_add_entity((0..HEAVY_ENTITY_COUNT).map(|_| heavy_bundle()));
    world
}
pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/shipyard", |b| {
        let world = heavy_world();
        let (mut positions, transforms) = world
            .borrow::<(ViewMut<PositionComponent>, View<TransformComponent>)>()
            .unwrap();
        b.iter(|| {
            (&mut positions, &transforms)
                .iter()
                .for_each(|(position, transform)| {
                    let mut matrix = transform.0;
                    for _ in 0..HEAVY_INVERT_COUNT {
                        matrix = matrix
                            .invert()
                            .expect("heavy matrix should remain invertible");
                    }
                    position.0 = matrix.transform_vector(position.0);
                });
            black_box(&world);
        });
    });
}

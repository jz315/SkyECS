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
            let mut checksum = 0_u64;
            (&mut positions, &transforms)
                .iter()
                .for_each(|(position, transform)| {
                    let mut matrix = transform.0;
                    for _ in 0..HEAVY_INVERT_COUNT {
                        matrix = matrix.inverse();
                    }
                    position.0 = matrix.transform_vector(position.0);
                    checksum = add_full_position_checksum(checksum, position);
                });
            black_box(checksum);
            black_box(&world);
        });
    });
}

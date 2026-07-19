use super::mixed_frame::warm_query;
use super::*;

fn heavy_world() -> World {
    let mut world = World::default();
    world.spawn_batch(SUITE_MASK, HEAVY_ENTITY_COUNT, |table, index| {
        let (transform, position, rotation, velocity) = heavy_bundle();
        table.transform[index] = transform;
        table.position[index] = position;
        table.rotation[index] = rotation;
        table.velocity[index] = velocity;
    });
    world
}
pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("heavy/freecs", |b| {
        let mut world = heavy_world();
        warm_query(&mut world, HEAVY_MASK);
        b.iter(|| {
            let mut checksum = 0_u64;
            world.for_each_mut(HEAVY_MASK, 0, |_entity, table, index| {
                let mut matrix = table.transform[index].0;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = matrix.inverse();
                }
                table.position[index].0 = matrix.transform_vector(table.position[index].0);
                checksum = add_full_position_checksum(checksum, &table.position[index]);
            });
            black_box(checksum);
            black_box(&world);
        });
    });
}

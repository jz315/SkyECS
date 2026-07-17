use super::*;

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/bevy"), |b| {
            let mut world = World::new();
            let entities: Vec<_> = (0..count)
                .map(|_| world.spawn(light_bundle()).id())
                .collect();
            let orders = deterministic_orders(&entities);
            let query = world.query::<&PositionComponent>();
            let mut order = 0;
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = query
                        .get_manual(&world, entity)
                        .expect("random-access entity must contain PositionComponent");
                    checksum = add_position_checksum(checksum, position);
                }
                black_box(checksum);
            });
        });
    }
}

use super::*;

fn reference_checksum(positions: &[&PositionComponent]) -> u64 {
    positions.iter().copied().fold(0_u64, add_position_checksum)
}

pub fn bench_entity_id_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/hecs"), |b| {
            let mut world = World::new();
            let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
            let orders = deterministic_orders(&entities);
            let mut query = PreparedQuery::<&PositionComponent>::default();
            let view = query.view_mut(&mut world);
            let mut order = 0;
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = view
                        .get(entity)
                        .expect("random-access entity must contain PositionComponent");
                    checksum = add_position_checksum(checksum, position);
                }
                black_box(checksum);
            });
        });
    }
}

pub fn bench_fixed_sequence_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (label, count) in [
        ("10k", SIMPLE_ENTITY_COUNT),
        ("100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("build_{label}/hecs"), |bencher| {
            let mut world = World::new();
            let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
            let orders = deterministic_orders(&entities);
            let mut query = PreparedQuery::<&PositionComponent>::default();
            let view = query.view_mut(&mut world);
            let mut order = 0;
            bencher.iter(|| {
                let plan: Vec<_> = orders[order % orders.len()]
                    .iter()
                    .map(|&entity| view.get(entity).unwrap())
                    .collect();
                order += 1;
                black_box(plan);
            });
        });

        group.bench_function(format!("steady_{label}/hecs"), |bencher| {
            let mut world = World::new();
            let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
            let orders = deterministic_orders(&entities);
            let mut query = PreparedQuery::<&PositionComponent>::default();
            let view = query.view_mut(&mut world);
            let plans: Vec<Vec<_>> = orders
                .iter()
                .map(|order| {
                    order
                        .iter()
                        .map(|&entity| view.get(entity).unwrap())
                        .collect()
                })
                .collect();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = reference_checksum(&plans[order % plans.len()]);
                order += 1;
                black_box(checksum);
            });
        });

        for repeats in [1_usize, 4, 16, 64] {
            group.bench_function(format!("amortized_{label}_x{repeats}/hecs"), |bencher| {
                let mut world = World::new();
                let entities: Vec<_> = (0..count).map(|_| world.spawn(light_bundle())).collect();
                let orders = deterministic_orders(&entities);
                let mut query = PreparedQuery::<&PositionComponent>::default();
                let view = query.view_mut(&mut world);
                let mut order = 0;
                bencher.iter(|| {
                    let plan: Vec<_> = orders[order % orders.len()]
                        .iter()
                        .map(|&entity| view.get(entity).unwrap())
                        .collect();
                    order += 1;
                    let mut checksum = 0_u64;
                    for _ in 0..repeats {
                        checksum = checksum.wrapping_add(reference_checksum(&plan));
                    }
                    black_box(checksum);
                });
            });
        }
    }
}

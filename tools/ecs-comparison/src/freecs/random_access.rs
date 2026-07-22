use super::structural_changes::spawn_light_batch;
use super::*;

fn reference_checksum(positions: &[&PositionComponent]) -> u64 {
    positions.iter().copied().fold(0_u64, add_position_checksum)
}

pub fn bench_entity_id_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/freecs"), |b| {
            let mut world = World::default();
            let entities = spawn_light_batch(&mut world, count);
            let orders = deterministic_orders(&entities);
            let mut order = 0;
            b.iter(|| {
                let entities = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &entity in entities {
                    let position = world
                        .get_position(entity)
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
        group.bench_function(format!("build_{label}/freecs"), |bencher| {
            let mut world = World::default();
            let entities = spawn_light_batch(&mut world, count);
            let orders = deterministic_orders(&entities);
            let mut order = 0;
            bencher.iter(|| {
                let plan: Vec<_> = orders[order % orders.len()]
                    .iter()
                    .map(|&entity| world.get_position(entity).unwrap())
                    .collect();
                order += 1;
                black_box(plan);
            });
        });

        group.bench_function(format!("steady_{label}/freecs"), |bencher| {
            let mut world = World::default();
            let entities = spawn_light_batch(&mut world, count);
            let orders = deterministic_orders(&entities);
            let plans: Vec<Vec<_>> = orders
                .iter()
                .map(|order| {
                    order
                        .iter()
                        .map(|&entity| world.get_position(entity).unwrap())
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
            group.bench_function(format!("amortized_{label}_x{repeats}/freecs"), |bencher| {
                let mut world = World::default();
                let entities = spawn_light_batch(&mut world, count);
                let orders = deterministic_orders(&entities);
                let mut order = 0;
                bencher.iter(|| {
                    let plan: Vec<_> = orders[order % orders.len()]
                        .iter()
                        .map(|&entity| world.get_position(entity).unwrap())
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

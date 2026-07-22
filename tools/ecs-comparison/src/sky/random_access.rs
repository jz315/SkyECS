use super::*;
use sky_ecs::{EntityAccessor, PreparedEntityAccess};

pub(super) fn random_world_and_orders(entity_count: usize) -> (World, Vec<Vec<EntityId>>) {
    let mut world = World::new();
    let entities: Vec<_> = (0..entity_count)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    let orders = deterministic_orders(&entities);
    (world, orders)
}

#[inline]
pub(super) fn accessor_checksum(
    positions: &EntityAccessor<'_, PositionComponent>,
    entities: &[EntityId],
) -> u64 {
    entities.iter().fold(0_u64, |checksum, &entity| {
        let position = positions
            .get(entity)
            .expect("random-access entity must contain PositionComponent");
        add_position_checksum(checksum, position)
    })
}

#[inline]
pub(super) fn prepared_checksum(positions: &PreparedEntityAccess<'_, PositionComponent>) -> u64 {
    positions.iter().fold(0_u64, add_position_checksum)
}

pub fn bench_entity_id_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/sky"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let positions = world.accessor::<PositionComponent>();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = accessor_checksum(&positions, &orders[order % orders.len()]);
                order += 1;
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
        group.bench_function(format!("build_{label}/sky"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let mut order = 0;
            bencher.iter(|| {
                let plan = world
                    .prepare_access::<PositionComponent>(&orders[order % orders.len()])
                    .expect("fixed sequence must be valid");
                order += 1;
                let _ = black_box(plan);
            });
        });

        group.bench_function(format!("steady_{label}/sky"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let plans: Vec<_> = orders
                .iter()
                .map(|entities| {
                    world
                        .prepare_access::<PositionComponent>(entities)
                        .expect("fixed sequence must be valid")
                })
                .collect();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = prepared_checksum(&plans[order % plans.len()]);
                order += 1;
                black_box(checksum);
            });
        });

        for repeats in [1_usize, 4, 16, 64] {
            group.bench_function(format!("amortized_{label}_x{repeats}/sky"), |bencher| {
                let (world, orders) = random_world_and_orders(count);
                let mut order = 0;
                bencher.iter(|| {
                    let plan = world
                        .prepare_access::<PositionComponent>(&orders[order % orders.len()])
                        .expect("fixed sequence must be valid");
                    order += 1;
                    let mut checksum = 0_u64;
                    for _ in 0..repeats {
                        checksum = checksum.wrapping_add(prepared_checksum(&plan));
                    }
                    black_box(checksum);
                });
            });
        }
    }
}

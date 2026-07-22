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

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/sky"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let plans: Vec<_> = orders
                .iter()
                .map(|entities| {
                    world
                        .prepare_access::<PositionComponent>(entities)
                        .expect("random-access fixture must be fully valid")
                })
                .collect();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = prepared_checksum(&plans[order % plans.len()]);
                order += 1;
                black_box(checksum);
            });
        });
    }
}

pub fn bench_random_access_api_candidates(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
    ] {
        group.bench_function(format!("{name}/entity_accessor"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let positions = world.accessor::<PositionComponent>();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = accessor_checksum(&positions, &orders[order % orders.len()]);
                order += 1;
                black_box(checksum);
            });
        });

        group.bench_function(format!("{name}/prepared_entity_access"), |bencher| {
            let (world, orders) = random_world_and_orders(count);
            let plans: Vec<_> = orders
                .iter()
                .map(|entities| {
                    world
                        .prepare_access::<PositionComponent>(entities)
                        .expect("random-access fixture must be fully valid")
                })
                .collect();
            let mut order = 0;
            bencher.iter(|| {
                let checksum = prepared_checksum(&plans[order % plans.len()]);
                order += 1;
                black_box(checksum);
            });
        });
    }
}

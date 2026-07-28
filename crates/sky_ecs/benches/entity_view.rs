use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::{
    EntityId, EntityView, PreparedEntityAccessor, PreparedEntityView, Res, ResMut, Update, World,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

const ENTITY_COUNT: usize = 10_000;

#[derive(Clone, Copy)]
struct TargetSlot(u32);

#[derive(Clone, Copy)]
struct Cooldown(u32);

#[allow(dead_code)]
struct Wide([u8; 4 * 1024]);

struct LookupOrder(Vec<EntityId>);

#[derive(Default)]
struct Checksum(u64);

fn fixture() -> (World, Vec<EntityId>) {
    let mut world = World::new();
    let entities: Vec<_> = (0..ENTITY_COUNT)
        .map(|index| {
            world.spawn((
                TargetSlot(((index * 17 + 11) % ENTITY_COUNT) as u32),
                Cooldown((index % 251) as u32),
            ))
        })
        .collect();
    let mut order = entities.clone();
    order.reverse();
    (world, order)
}

fn scheduled_lookup(
    order: Res<LookupOrder>,
    mut entities: EntityView<(&'static TargetSlot, &'static mut Cooldown)>,
    mut checksum: ResMut<Checksum>,
) {
    for &entity in &order.0 {
        let (target, cooldown) = entities.get_mut(entity).unwrap();
        cooldown.0 = cooldown.0.saturating_sub(1);
        checksum.0 = checksum.0.wrapping_add(target.0 as u64 ^ cooldown.0 as u64);
    }
}

fn bench_entity_views(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("entity_view_api_10k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30);

    group.bench_function("world_get_pair", |bencher| {
        let (mut world, order) = fixture();
        bencher.iter(|| {
            let mut checksum = 0_u64;
            for &entity in &order {
                let target = world.get::<TargetSlot>(entity).unwrap().0;
                let cooldown = world.get_mut::<Cooldown>(entity).unwrap();
                cooldown.0 = cooldown.0.saturating_sub(1);
                checksum = checksum.wrapping_add(target as u64 ^ cooldown.0 as u64);
            }
            black_box(checksum);
        });
    });

    group.bench_function("split_entity_accessors", |bencher| {
        let (mut world, order) = fixture();
        bencher.iter(|| {
            let mut checksum = 0_u64;
            {
                let targets = world.accessor::<TargetSlot>();
                for &entity in &order {
                    checksum = checksum.wrapping_add(targets.get(entity).unwrap().0 as u64);
                }
            }
            {
                let mut cooldowns = world.accessor_mut::<Cooldown>();
                for &entity in &order {
                    let cooldown = cooldowns.get_mut(entity).unwrap();
                    cooldown.0 = cooldown.0.saturating_sub(1);
                    checksum ^= cooldown.0 as u64;
                }
            }
            black_box(checksum);
        });
    });

    group.bench_function("prepared_entity_view", |bencher| {
        let (mut world, order) = fixture();
        let mut prepared =
            PreparedEntityView::<(&'static TargetSlot, &'static mut Cooldown)>::new();
        bencher.iter(|| {
            let mut entities = prepared.bind_mut(&mut world);
            let mut checksum = 0_u64;
            for &entity in &order {
                let (target, cooldown) = entities.get_mut(entity).unwrap();
                cooldown.0 = cooldown.0.saturating_sub(1);
                checksum = checksum.wrapping_add(target.0 as u64 ^ cooldown.0 as u64);
            }
            black_box(checksum);
        });
    });

    group.bench_function("scheduled_entity_view", |bencher| {
        let (mut world, order) = fixture();
        world.insert_resource(LookupOrder(order));
        world.insert_resource(Checksum::default());
        world.stage(Update).add(scheduled_lookup);
        bencher.iter(|| black_box(world.tick_with_delta(0.0).unwrap()));
    });

    group.finish();

    let mut prepare = criterion.benchmark_group("entity_view_route_prepare");
    prepare.bench_function("stable_world", |bencher| {
        let (world, order) = fixture();
        let mut view = PreparedEntityView::<&TargetSlot>::new();
        let _ = view.bind(&world);
        bencher.iter(|| {
            black_box(view.bind(&world).get(order[0]));
        });
    });
    prepare.bench_function("row_churn_without_chunk_change", |bencher| {
        let mut world = World::new();
        let survivor = world.spawn((TargetSlot(1),));
        let mut view = PreparedEntityView::<&TargetSlot>::new();
        let _ = view.bind(&world);
        bencher.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let temporary = world.spawn((TargetSlot(2),));
                let start = Instant::now();
                black_box(view.bind(&world).get(survivor));
                elapsed += start.elapsed();
                assert!(world.despawn(temporary));
            }
            elapsed
        });
    });
    for (name, shrink) in [("route_peak", false), ("route_peak_then_shrink", true)] {
        prepare.bench_function(format!("entity_accessor_construct/{name}"), |bencher| {
            let mut world = World::new();
            let survivor = world.spawn((TargetSlot(1),));
            let temporary: Vec<_> = (0..160)
                .map(|value| world.spawn((TargetSlot(value), Wide([value as u8; 4 * 1024]))))
                .collect();
            for entity in temporary {
                assert!(world.despawn(entity));
            }
            if shrink {
                world.shrink_route_tables();
            }
            bencher.iter(|| {
                let accessor = world.accessor::<TargetSlot>();
                black_box(accessor.get(survivor));
            });
        });
    }
    prepare.finish();
}

fn bench_single_component_accessors(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("entity_accessor_api_10k");
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30);

    group.bench_function("one_shot_entity_accessor", |bencher| {
        let (world, order) = fixture();
        bencher.iter(|| {
            let targets = world.accessor::<TargetSlot>();
            let checksum = order.iter().fold(0_u64, |checksum, &entity| {
                checksum.wrapping_add(targets.get(entity).unwrap().0 as u64)
            });
            black_box(checksum);
        });
    });

    group.bench_function("prepared_entity_accessor", |bencher| {
        let (world, order) = fixture();
        let mut prepared = PreparedEntityAccessor::<TargetSlot>::new();
        bencher.iter(|| {
            let targets = prepared.bind(&world);
            let checksum = order.iter().fold(0_u64, |checksum, &entity| {
                checksum.wrapping_add(targets.get(entity).unwrap().0 as u64)
            });
            black_box(checksum);
        });
    });

    group.bench_function("prepared_entity_view", |bencher| {
        let (world, order) = fixture();
        let mut prepared = PreparedEntityView::<&TargetSlot>::new();
        bencher.iter(|| {
            let targets = prepared.bind(&world);
            let checksum = order.iter().fold(0_u64, |checksum, &entity| {
                checksum.wrapping_add(targets.get(entity).unwrap().0 as u64)
            });
            black_box(checksum);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_views,
    bench_single_component_accessors
);
criterion_main!(benches);

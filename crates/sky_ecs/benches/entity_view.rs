use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::{EntityId, EntityView, PreparedEntityView, Res, ResMut, Update, World};
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNT: usize = 10_000;

#[derive(Clone, Copy)]
struct TargetSlot(u32);

#[derive(Clone, Copy)]
struct Cooldown(u32);

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
}

criterion_group!(benches, bench_entity_views);
criterion_main!(benches);

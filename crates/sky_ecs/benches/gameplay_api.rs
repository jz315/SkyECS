#[path = "gameplay_api/certification.rs"]
mod certification;

use criterion::Criterion;
use sky_ecs::{EntityId, PreparedEntityView, PreparedQuery, World};
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNT: usize = 32_768;
const AI_COUNT: usize = 2_048;

#[derive(Clone, Copy)]
struct Position(f32);

#[derive(Clone, Copy)]
struct Velocity(f32);

#[derive(Clone, Copy)]
struct TargetSlot(u32);

#[derive(Clone, Copy)]
struct Cooldown(u32);

pub(crate) struct GameplayFixture {
    world: World,
    entities: Vec<EntityId>,
    ai_slots: Vec<usize>,
    targets: Vec<EntityId>,
    movement: PreparedQuery<(&'static mut Position, &'static Velocity)>,
    ai: PreparedEntityView<(&'static TargetSlot, &'static mut Cooldown)>,
    positions: PreparedEntityView<&'static Position>,
    checksum: u64,
}

impl GameplayFixture {
    pub(crate) fn new() -> Self {
        let mut world = World::new();
        let entities: Vec<_> = (0..ENTITY_COUNT)
            .map(|index| {
                world.spawn((
                    Position(index as f32),
                    Velocity((index % 7) as f32 * 0.125 + 0.25),
                    TargetSlot(((index * 17 + 23) % ENTITY_COUNT) as u32),
                    Cooldown((index % 251) as u32),
                ))
            })
            .collect();
        let ai_slots = (0..AI_COUNT).map(|index| index * 4).collect();
        Self {
            world,
            entities,
            ai_slots,
            targets: Vec::with_capacity(AI_COUNT),
            movement: PreparedQuery::new(),
            ai: PreparedEntityView::new(),
            positions: PreparedEntityView::new(),
            checksum: 0,
        }
    }

    pub(crate) fn iteration_closure(&mut self) {
        self.movement
            .for_each_chunk(&mut self.world, |(positions, velocities)| {
                move_chunk(positions, velocities);
            });
    }

    pub(crate) fn iteration_function(&mut self) {
        self.movement.for_each_chunk_fn(&mut self.world, move_chunk);
    }

    pub(crate) fn ai_world_get_pair(&mut self) {
        self.targets.clear();
        for &slot in &self.ai_slots {
            let entity = self.entities[slot];
            let target = self.world.get::<TargetSlot>(entity).unwrap().0 as usize;
            self.targets.push(self.entities[target]);
            let cooldown = self.world.get_mut::<Cooldown>(entity).unwrap();
            cooldown.0 = cooldown.0.saturating_sub(1);
        }
    }

    pub(crate) fn ai_split_accessors(&mut self) {
        self.targets.clear();
        {
            let targets = self.world.accessor::<TargetSlot>();
            for &slot in &self.ai_slots {
                let target = targets.get(self.entities[slot]).unwrap().0 as usize;
                self.targets.push(self.entities[target]);
            }
        }
        let mut cooldowns = self.world.accessor_mut::<Cooldown>();
        for &slot in &self.ai_slots {
            let cooldown = cooldowns.get_mut(self.entities[slot]).unwrap();
            cooldown.0 = cooldown.0.saturating_sub(1);
        }
    }

    pub(crate) fn ai_prepared_entity_view(&mut self) {
        self.targets.clear();
        let mut ai = self.ai.bind_mut(&mut self.world);
        for &slot in &self.ai_slots {
            let (target, cooldown) = ai.get_mut(self.entities[slot]).unwrap();
            self.targets.push(self.entities[target.0 as usize]);
            cooldown.0 = cooldown.0.saturating_sub(1);
        }
    }

    pub(crate) fn positions_world_get(&mut self) {
        for &entity in &self.targets {
            let position = self.world.get::<Position>(entity).unwrap();
            self.checksum = self.checksum.wrapping_add(position.0.to_bits() as u64);
        }
    }

    pub(crate) fn positions_accessor(&mut self) {
        let positions = self.world.accessor::<Position>();
        for &entity in &self.targets {
            let position = positions.get(entity).unwrap();
            self.checksum = self.checksum.wrapping_add(position.0.to_bits() as u64);
        }
    }

    pub(crate) fn positions_prepared_entity_view(&mut self) {
        let positions = self.positions.bind(&self.world);
        for &entity in &self.targets {
            let position = positions.get(entity).unwrap();
            self.checksum = self.checksum.wrapping_add(position.0.to_bits() as u64);
        }
    }

    pub(crate) fn checksum(&self) -> u64 {
        self.checksum
    }
}

#[inline(never)]
fn move_chunk(positions: &mut [Position], velocities: &[Velocity]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

fn configured_group<'a>(
    criterion: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = criterion.benchmark_group(name);
    group
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30);
    group
}

fn bench_iteration(criterion: &mut Criterion) {
    let mut group = configured_group(criterion, "gameplay_api_iteration");
    group.bench_function("chunk_closure", |bencher| {
        let mut fixture = GameplayFixture::new();
        bencher.iter(|| {
            fixture.iteration_closure();
            black_box(&fixture);
        });
    });
    group.bench_function("chunk_function", |bencher| {
        let mut fixture = GameplayFixture::new();
        bencher.iter(|| {
            fixture.iteration_function();
            black_box(&fixture);
        });
    });
    group.finish();
}

fn bench_ai(criterion: &mut Criterion) {
    let mut group = configured_group(criterion, "gameplay_api_ai_source");
    for (name, run) in [
        (
            "world_get_pair",
            GameplayFixture::ai_world_get_pair as fn(&mut GameplayFixture),
        ),
        ("split_accessors", GameplayFixture::ai_split_accessors),
        (
            "prepared_entity_view",
            GameplayFixture::ai_prepared_entity_view,
        ),
    ] {
        group.bench_function(name, move |bencher| {
            let mut fixture = GameplayFixture::new();
            bencher.iter(|| {
                run(&mut fixture);
                black_box(&fixture);
            });
        });
    }
    group.finish();
}

fn bench_positions(criterion: &mut Criterion) {
    let mut group = configured_group(criterion, "gameplay_api_target_position");
    for (name, run) in [
        (
            "world_get",
            GameplayFixture::positions_world_get as fn(&mut GameplayFixture),
        ),
        ("entity_accessor", GameplayFixture::positions_accessor),
        (
            "prepared_entity_view",
            GameplayFixture::positions_prepared_entity_view,
        ),
    ] {
        group.bench_function(name, move |bencher| {
            let mut fixture = GameplayFixture::new();
            fixture.ai_prepared_entity_view();
            bencher.iter(|| {
                run(&mut fixture);
                black_box(fixture.checksum);
            });
        });
    }
    group.finish();
}

fn bench_frame(criterion: &mut Criterion) {
    let mut group = configured_group(criterion, "gameplay_api_full_frame");
    for (name, run) in [
        (
            "world_get_baseline",
            certification::FrameSelection::world_get_baseline(),
        ),
        (
            "split_accessor_path",
            certification::FrameSelection::split_accessor_path(),
        ),
        (
            "all_prepared_views",
            certification::FrameSelection::all_prepared_views(),
        ),
        (
            "selected_production_path",
            certification::FrameSelection::production(),
        ),
    ] {
        group.bench_function(name, move |bencher| {
            let mut fixture = GameplayFixture::new();
            bencher.iter(|| {
                run.run(&mut fixture);
                black_box(&fixture);
            });
        });
    }
    group.finish();
}

fn main() {
    if std::env::var_os("SKY_ECS_CERTIFY_GAMEPLAY_API").is_some() {
        certification::run();
        return;
    }

    let mut criterion = Criterion::default().configure_from_args();
    bench_iteration(&mut criterion);
    bench_ai(&mut criterion);
    bench_positions(&mut criterion);
    bench_frame(&mut criterion);
    criterion.final_summary();
}

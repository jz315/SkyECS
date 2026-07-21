use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use sky_ecs::dynamic::{DynamicBundle, WorldDynamicExt};
use sky_ecs::{EntityId, PreparedQuery, World};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SkyIterationApi {
    ChunkClosure,
    ChunkFunction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SkyLookupApi {
    WorldGet,
    ComponentAccessor,
}

pub(super) const SELECTED_ITERATION_API: SkyIterationApi = SkyIterationApi::ChunkClosure;
pub(super) const SELECTED_LOOKUP_API: SkyLookupApi = SkyLookupApi::ComponentAccessor;

pub(super) struct SkyGameplayWorld {
    world: World,
    entities: Vec<EntityId>,
    generations: Vec<u32>,
    target_entities: Vec<EntityId>,
    movement: PreparedQuery<(&'static mut PositionComponent, &'static VelocityComponent)>,
    enemies: PreparedQuery<(&'static mut Health, &'static Damage)>,
    allies: PreparedQuery<(&'static mut Health, &'static Regen)>,
    lifetimes: PreparedQuery<&'static mut Lifetime>,
    ai_lookup_checksum: u64,
}

impl SkyGameplayWorld {
    pub(super) fn new(trace: &GameplayTrace) -> Self {
        let mut world = World::new();
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }

        let mut entities = Vec::with_capacity(GAMEPLAY_ENTITY_COUNT);
        for (slot, &stunned) in initially_stunned.iter().enumerate() {
            let bundle = dynamic_gameplay_bundle(gameplay_entity_values(slot, 0, stunned), slot);
            entities.push(
                world
                    .spawn_dynamic(bundle)
                    .expect("gameplay bundle components must be unique"),
            );
        }

        let mut movement = PreparedQuery::<(&mut PositionComponent, &VelocityComponent)>::new();
        let mut enemies = PreparedQuery::<(&mut Health, &Damage)>::new();
        let mut allies = PreparedQuery::<(&mut Health, &Regen)>::new();
        let mut lifetimes = PreparedQuery::<&mut Lifetime>::new();
        assert_eq!(movement.count(&world), GAMEPLAY_MOVING_COUNT);
        assert_eq!(enemies.count(&world), GAMEPLAY_ENEMY_COUNT);
        assert_eq!(
            allies.count(&world),
            GAMEPLAY_COMBAT_COUNT - GAMEPLAY_ENEMY_COUNT
        );
        assert_eq!(lifetimes.count(&world), GAMEPLAY_LIFETIME_COUNT);
        assert_eq!(world.entity_count(), GAMEPLAY_ENTITY_COUNT);
        assert!(world.archetype_count() >= 32);

        Self {
            world,
            entities,
            generations: vec![0; GAMEPLAY_ENTITY_COUNT],
            target_entities: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            movement,
            enemies,
            allies,
            lifetimes,
            ai_lookup_checksum: 0,
        }
    }

    pub(super) fn run_frame(&mut self, frame: &GameplayFrame) {
        self.run_frame_with_apis(frame, SELECTED_ITERATION_API, SELECTED_LOOKUP_API);
    }

    pub(super) fn run_frame_with_apis(
        &mut self,
        frame: &GameplayFrame,
        iteration_api: SkyIterationApi,
        lookup_api: SkyLookupApi,
    ) {
        match iteration_api {
            SkyIterationApi::ChunkClosure => {
                self.movement
                    .for_each_chunk(&mut self.world, |(positions, velocities)| {
                        move_chunk(positions, velocities);
                    });
                self.enemies
                    .for_each_chunk(&mut self.world, |(health, damage)| {
                        damage_chunk(health, damage);
                    });
                self.allies
                    .for_each_chunk(&mut self.world, |(health, regen)| {
                        regen_chunk(health, regen);
                    });
                self.lifetimes
                    .for_each_chunk(&mut self.world, |lifetimes| lifetime_chunk(lifetimes));
            }
            SkyIterationApi::ChunkFunction => {
                self.movement.for_each_chunk_fn(&mut self.world, move_chunk);
                self.enemies
                    .for_each_chunk_fn(&mut self.world, damage_chunk);
                self.allies.for_each_chunk_fn(&mut self.world, regen_chunk);
                self.lifetimes
                    .for_each_chunk(&mut self.world, lifetime_chunk);
            }
        }

        self.target_entities.clear();
        for &slot in frame.ai_slots.iter() {
            let ai_entity = self.entities[slot];
            let target_slot = self
                .world
                .get::<TargetSlot>(ai_entity)
                .expect("AI entity must have TargetSlot")
                .0 as usize;
            self.target_entities.push(self.entities[target_slot]);
            let cooldown = self
                .world
                .get_mut::<Cooldown>(ai_entity)
                .expect("AI entity must have Cooldown");
            cooldown.0 = cooldown.0.saturating_sub(1);
        }

        match lookup_api {
            SkyLookupApi::WorldGet => {
                for (&slot, &target) in frame.ai_slots.iter().zip(&self.target_entities) {
                    let position = self
                        .world
                        .get::<PositionComponent>(target)
                        .expect("AI target must have PositionComponent");
                    self.ai_lookup_checksum = gameplay_mix_checksum(
                        self.ai_lookup_checksum,
                        slot as u64,
                        position.0.x.to_bits() as u64,
                    );
                }
            }
            SkyLookupApi::ComponentAccessor => {
                let positions = self.world.accessor::<PositionComponent>();
                for (&slot, &target) in frame.ai_slots.iter().zip(&self.target_entities) {
                    let position = positions
                        .get(target)
                        .expect("AI target must have PositionComponent");
                    self.ai_lookup_checksum = gameplay_mix_checksum(
                        self.ai_lookup_checksum,
                        slot as u64,
                        position.0.x.to_bits() as u64,
                    );
                }
            }
        }

        for &slot in frame.remove_stunned.iter() {
            let removed = self.world.remove::<Stunned>(self.entities[slot]);
            assert!(removed, "removing absent Stunned at slot {slot}");
        }
        for &slot in frame.add_stunned.iter() {
            let inserted = self.world.insert(self.entities[slot], Stunned);
            assert!(inserted, "adding duplicate Stunned at slot {slot}");
        }

        for &slot in frame.recycle_projectiles.iter() {
            assert!(self.world.despawn(self.entities[slot]));
            let generation = self.generations[slot].wrapping_add(1);
            self.generations[slot] = generation;
            self.entities[slot] = spawn_projectile(&mut self.world, slot, generation);
        }
        debug_assert_eq!(self.world.entity_count(), GAMEPLAY_ENTITY_COUNT);
    }

    pub(super) fn digest(&self) -> GameplayDigest {
        let mut moving_count = 0;
        let mut health_count = 0;
        let mut lifetime_count = 0;
        let mut stunned_count = 0;
        let mut position_checksum = 0;
        let mut health_checksum = 0;
        let mut lifetime_checksum = 0;
        let mut generation_checksum = 0;

        for (slot, &entity) in self.entities.iter().enumerate() {
            moving_count += usize::from(self.world.get::<VelocityComponent>(entity).is_some());
            health_count += usize::from(self.world.get::<Health>(entity).is_some());
            lifetime_count += usize::from(self.world.get::<Lifetime>(entity).is_some());
            stunned_count += usize::from(self.world.get::<Stunned>(entity).is_some());

            let position = self
                .world
                .get::<PositionComponent>(entity)
                .expect("every gameplay entity must have PositionComponent");
            position_checksum = gameplay_mix_checksum(
                position_checksum,
                slot as u64,
                (position.0.x.to_bits() as u64)
                    ^ ((position.0.y.to_bits() as u64) << 1)
                    ^ ((position.0.z.to_bits() as u64) << 2),
            );
            if let Some(health) = self.world.get::<Health>(entity) {
                health_checksum =
                    gameplay_mix_checksum(health_checksum, slot as u64, health.0.to_bits() as u64);
            }
            if let Some(lifetime) = self.world.get::<Lifetime>(entity) {
                lifetime_checksum =
                    gameplay_mix_checksum(lifetime_checksum, slot as u64, lifetime.0 as u64);
            }
            generation_checksum = gameplay_mix_checksum(
                generation_checksum,
                slot as u64,
                self.generations[slot] as u64,
            );
        }

        GameplayDigest {
            entity_count: self.world.entity_count(),
            moving_count,
            health_count,
            lifetime_count,
            stunned_count,
            position_checksum,
            health_checksum,
            lifetime_checksum,
            generation_checksum,
            ai_lookup_checksum: self.ai_lookup_checksum,
        }
    }
}

fn dynamic_gameplay_bundle(values: GameplayEntityValues, slot: usize) -> DynamicBundle {
    let mut bundle = DynamicBundle::new().with(values.position);
    if let Some(value) = values.velocity {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.health {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.damage {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.regen {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.enemy {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.ally {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.lifetime {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.target {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.cooldown {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.owner {
        bundle = bundle.with(value);
    }
    if let Some(value) = values.stunned {
        bundle = bundle.with(value);
    }
    match gameplay_slot_spec(slot).variant {
        0 => {}
        1 => bundle = bundle.with(TagA),
        2 => bundle = bundle.with(TagB),
        3 => bundle = bundle.with(TagA).with(TagB),
        _ => unreachable!(),
    }
    bundle
}

fn spawn_projectile(world: &mut World, slot: usize, generation: u32) -> EntityId {
    let values = gameplay_entity_values(slot, generation, false);
    let position = values.position;
    let velocity = values.velocity.expect("projectile velocity");
    let damage = values.damage.expect("projectile damage");
    let lifetime = values.lifetime.expect("projectile lifetime");
    let owner = values.owner.expect("projectile owner");
    match gameplay_slot_spec(slot).variant {
        0 => world.spawn((position, velocity, damage, lifetime, owner)),
        1 => world.spawn((position, velocity, damage, lifetime, owner, TagA)),
        2 => world.spawn((position, velocity, damage, lifetime, owner, TagB)),
        3 => world.spawn((position, velocity, damage, lifetime, owner, TagA, TagB)),
        _ => unreachable!(),
    }
}

#[inline(never)]
fn move_chunk(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

#[inline(never)]
fn damage_chunk(health: &mut [Health], damage: &[Damage]) {
    for (health, damage) in health.iter_mut().zip(damage) {
        health.0 -= damage.0;
    }
}

#[inline(never)]
fn regen_chunk(health: &mut [Health], regen: &[Regen]) {
    for (health, regen) in health.iter_mut().zip(regen) {
        health.0 += regen.0;
    }
}

#[inline(never)]
fn lifetime_chunk(lifetimes: &mut [Lifetime]) {
    for lifetime in lifetimes {
        lifetime.0 = lifetime.0.saturating_sub(1);
        if lifetime.0 == 0 {
            lifetime.0 = 256;
        }
    }
}

pub fn validate_gameplay_contract() {
    let trace = GameplayTrace::standard();
    let mut reference = GameplayReference::new(&trace);
    reference.run_trace(&trace);
    assert_eq!(reference.digest(), GAMEPLAY_CANONICAL_DIGEST);

    for (iteration_api, lookup_api) in [
        (SkyIterationApi::ChunkClosure, SkyLookupApi::WorldGet),
        (
            SkyIterationApi::ChunkFunction,
            SkyLookupApi::ComponentAccessor,
        ),
    ] {
        let mut gameplay = SkyGameplayWorld::new(&trace);
        for frame in trace.frames() {
            gameplay.run_frame_with_apis(frame, iteration_api, lookup_api);
        }
        assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
    }
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/sky", |bencher| {
        let trace = GameplayTrace::standard();
        let mut gameplay = SkyGameplayWorld::new(&trace);
        let mut frame = 0;
        bencher.iter(|| {
            gameplay.run_frame(&trace.frames()[frame]);
            frame = (frame + 1) % GAMEPLAY_FRAME_COUNT;
            black_box(&gameplay.world);
        });
    });
}

pub fn bench_gameplay_api_candidates(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, iteration, lookup) in [
        (
            "iteration_chunk_closure",
            SkyIterationApi::ChunkClosure,
            SkyLookupApi::ComponentAccessor,
        ),
        (
            "iteration_chunk_function",
            SkyIterationApi::ChunkFunction,
            SkyLookupApi::ComponentAccessor,
        ),
        (
            "lookup_world_get",
            SkyIterationApi::ChunkFunction,
            SkyLookupApi::WorldGet,
        ),
        (
            "lookup_component_accessor",
            SkyIterationApi::ChunkFunction,
            SkyLookupApi::ComponentAccessor,
        ),
    ] {
        group.bench_function(name, move |bencher| {
            let trace = GameplayTrace::standard();
            let mut gameplay = SkyGameplayWorld::new(&trace);
            let mut frame = 0;
            bencher.iter(|| {
                gameplay.run_frame_with_apis(&trace.frames()[frame], iteration, lookup);
                frame = (frame + 1) % GAMEPLAY_FRAME_COUNT;
                black_box(&gameplay.world);
            });
        });
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct SkyApiCandidateResult {
    pub first: &'static str,
    pub first_ns_per_frame: f64,
    pub second: &'static str,
    pub second_ns_per_frame: f64,
    pub winner: &'static str,
    pub difference_percent: f64,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct SkyGameplayApiCertification {
    pub rotations: usize,
    pub traces_per_rotation: usize,
    pub frames_per_trace: usize,
    pub iteration: SkyApiCandidateResult,
    pub lookup: SkyApiCandidateResult,
}

pub fn certify_gameplay_apis(
    rotations: usize,
    traces_per_rotation: usize,
) -> SkyGameplayApiCertification {
    assert!(rotations > 0);
    assert!(traces_per_rotation > 0);
    let trace = GameplayTrace::standard();
    let iteration = paired_certification(
        &trace,
        rotations,
        traces_per_rotation,
        "PreparedQuery::for_each_chunk",
        SkyIterationApi::ChunkClosure,
        SkyLookupApi::ComponentAccessor,
        "PreparedQuery::for_each_chunk_fn",
        SkyIterationApi::ChunkFunction,
        SkyLookupApi::ComponentAccessor,
    );
    let lookup = paired_certification(
        &trace,
        rotations,
        traces_per_rotation,
        "World::get",
        SELECTED_ITERATION_API,
        SkyLookupApi::WorldGet,
        "ComponentAccessor::get",
        SELECTED_ITERATION_API,
        SkyLookupApi::ComponentAccessor,
    );
    SkyGameplayApiCertification {
        rotations,
        traces_per_rotation,
        frames_per_trace: GAMEPLAY_FRAME_COUNT,
        iteration,
        lookup,
    }
}

#[allow(clippy::too_many_arguments)]
fn paired_certification(
    trace: &GameplayTrace,
    rotations: usize,
    traces_per_rotation: usize,
    first_name: &'static str,
    first_iteration: SkyIterationApi,
    first_lookup: SkyLookupApi,
    second_name: &'static str,
    second_iteration: SkyIterationApi,
    second_lookup: SkyLookupApi,
) -> SkyApiCandidateResult {
    let mut first = SkyGameplayWorld::new(trace);
    let mut second = SkyGameplayWorld::new(trace);
    for _ in 0..2 {
        run_candidate_trace(&mut first, trace, first_iteration, first_lookup);
        run_candidate_trace(&mut second, trace, second_iteration, second_lookup);
    }

    let mut first_elapsed = Duration::ZERO;
    let mut second_elapsed = Duration::ZERO;
    for rotation in 0..rotations {
        for _ in 0..traces_per_rotation {
            if rotation % 2 == 0 {
                first_elapsed +=
                    time_candidate_trace(&mut first, trace, first_iteration, first_lookup);
                second_elapsed +=
                    time_candidate_trace(&mut second, trace, second_iteration, second_lookup);
            } else {
                second_elapsed +=
                    time_candidate_trace(&mut second, trace, second_iteration, second_lookup);
                first_elapsed +=
                    time_candidate_trace(&mut first, trace, first_iteration, first_lookup);
            }
        }
    }

    let frame_count = (rotations * traces_per_rotation * GAMEPLAY_FRAME_COUNT) as f64;
    let first_ns = first_elapsed.as_nanos() as f64 / frame_count;
    let second_ns = second_elapsed.as_nanos() as f64 / frame_count;
    let (winner, faster, slower) = if first_ns <= second_ns {
        (first_name, first_ns, second_ns)
    } else {
        (second_name, second_ns, first_ns)
    };
    SkyApiCandidateResult {
        first: first_name,
        first_ns_per_frame: first_ns,
        second: second_name,
        second_ns_per_frame: second_ns,
        winner,
        difference_percent: (slower / faster - 1.0) * 100.0,
    }
}

fn time_candidate_trace(
    gameplay: &mut SkyGameplayWorld,
    trace: &GameplayTrace,
    iteration: SkyIterationApi,
    lookup: SkyLookupApi,
) -> Duration {
    let start = Instant::now();
    run_candidate_trace(gameplay, trace, iteration, lookup);
    start.elapsed()
}

fn run_candidate_trace(
    gameplay: &mut SkyGameplayWorld,
    trace: &GameplayTrace,
    iteration: SkyIterationApi,
    lookup: SkyLookupApi,
) {
    for frame in trace.frames() {
        gameplay.run_frame_with_apis(frame, iteration, lookup);
    }
    black_box(&gameplay.world);
}

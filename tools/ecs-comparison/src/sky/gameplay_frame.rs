use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use sky_ecs::dynamic::{DynamicBundle, WorldDynamicExt};
use sky_ecs::{EntityId, PreparedEntityView, PreparedQuery, World};

pub(super) struct SkyGameplayWorld {
    world: World,
    entities: Vec<EntityId>,
    generations: Vec<u32>,
    target_entities: Vec<EntityId>,
    movement: PreparedQuery<(&'static mut PositionComponent, &'static VelocityComponent)>,
    enemies: PreparedQuery<(&'static mut Health, &'static Damage)>,
    allies: PreparedQuery<(&'static mut Health, &'static Regen)>,
    lifetimes: PreparedQuery<&'static mut Lifetime>,
    ai: PreparedEntityView<(&'static TargetSlot, &'static mut Cooldown)>,
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
            ai: PreparedEntityView::new(),
            ai_lookup_checksum: 0,
        }
    }

    pub(super) fn run_frame(&mut self, frame: &GameplayFrame) {
        self.run_iteration_phase();
        self.run_ai_source_phase(frame);
        self.run_target_position_phase(frame);
        self.run_status_transition_phase(frame);
        self.run_projectile_recycle_phase(frame);
    }

    pub(super) fn run_iteration_phase(&mut self) {
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
            .for_each_chunk(&mut self.world, lifetime_chunk);
    }

    pub(super) fn run_ai_source_phase(&mut self, frame: &GameplayFrame) {
        self.target_entities.clear();
        let mut ai = self.ai.bind_mut(&mut self.world);
        for &slot in frame.ai_slots.iter() {
            let (target, cooldown) = ai
                .get_mut(self.entities[slot])
                .expect("AI entity must have TargetSlot and Cooldown");
            self.target_entities.push(self.entities[target.0 as usize]);
            cooldown.0 = cooldown.0.saturating_sub(1);
        }
    }

    pub(super) fn run_target_position_phase(&mut self, frame: &GameplayFrame) {
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

    pub(super) fn run_status_transition_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.remove_stunned.iter() {
            let removed = self.world.remove::<Stunned>(self.entities[slot]);
            assert!(removed, "removing absent Stunned at slot {slot}");
        }
        for &slot in frame.add_stunned.iter() {
            let inserted = self.world.insert(self.entities[slot], Stunned);
            assert!(inserted, "adding duplicate Stunned at slot {slot}");
        }
    }

    pub(super) fn run_projectile_recycle_phase(&mut self, frame: &GameplayFrame) {
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

    let mut gameplay = SkyGameplayWorld::new(&trace);
    for frame in trace.frames() {
        gameplay.run_frame(frame);
    }
    assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
}

impl GameplayPhaseAdapter for SkyGameplayWorld {
    fn run_phase(&mut self, phase: GameplayPhase, frame: &GameplayFrame) {
        match phase {
            GameplayPhase::Iteration => self.run_iteration_phase(),
            GameplayPhase::AiSourceLookup => self.run_ai_source_phase(frame),
            GameplayPhase::TargetPositionLookup => self.run_target_position_phase(frame),
            GameplayPhase::StatusTransition => self.run_status_transition_phase(frame),
            GameplayPhase::ProjectileRecycle => self.run_projectile_recycle_phase(frame),
        }
    }

    fn digest(&self) -> GameplayDigest {
        SkyGameplayWorld::digest(self)
    }
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_full_gameplay_frames(
        group,
        "sky",
        SkyGameplayWorld::new,
        SkyGameplayWorld::run_frame,
    );
}

pub fn bench_gameplay_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_gameplay_phases(group, "sky", SkyGameplayWorld::new);
}

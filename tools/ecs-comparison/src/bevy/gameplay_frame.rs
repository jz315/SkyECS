use crate::common::*;
use bevy_ecs::{entity::Entity, query::QueryState, world::World};
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::collections::HashSet;

pub(super) struct BevyGameplayWorld {
    world: World,
    entities: Vec<Entity>,
    generations: Vec<u32>,
    target_entities: Vec<Entity>,
    target_slots: Vec<usize>,
    movement: QueryState<(&'static mut PositionComponent, &'static VelocityComponent)>,
    enemies: QueryState<(&'static mut Health, &'static Damage)>,
    allies: QueryState<(&'static mut Health, &'static Regen)>,
    lifetimes: QueryState<&'static mut Lifetime>,
    ai: QueryState<(&'static TargetSlot, &'static mut Cooldown)>,
    positions: QueryState<&'static PositionComponent>,
    ai_lookup_checksum: u64,
    cooldown_trace_checksum: u64,
}

impl BevyGameplayWorld {
    pub(super) fn new(trace: &GameplayTrace) -> Self {
        let mut world = World::new();
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }

        let mut entities = Vec::with_capacity(GAMEPLAY_ENTITY_COUNT);
        for (slot, &stunned) in initially_stunned.iter().enumerate() {
            let entity = world.spawn_empty().id();
            insert_gameplay_components(
                &mut world,
                entity,
                gameplay_entity_values(slot, 0, stunned),
                slot,
            );
            entities.push(entity);
        }

        let movement = world.query::<(&mut PositionComponent, &VelocityComponent)>();
        let enemies = world.query::<(&mut Health, &Damage)>();
        let allies = world.query::<(&mut Health, &Regen)>();
        let lifetimes = world.query::<&mut Lifetime>();
        let ai = world.query::<(&TargetSlot, &mut Cooldown)>();
        let positions = world.query::<&PositionComponent>();

        Self {
            world,
            entities,
            generations: vec![0; GAMEPLAY_ENTITY_COUNT],
            target_entities: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            target_slots: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            movement,
            enemies,
            allies,
            lifetimes,
            ai,
            positions,
            ai_lookup_checksum: 0,
            cooldown_trace_checksum: 0,
        }
    }

    pub(super) fn run_frame(&mut self, frame: &GameplayFrame) {
        self.run_iteration_phase();
        self.run_ai_source_phase(frame);
        self.run_target_position_phase(frame);
        self.run_status_transition_phase(frame);
        self.run_projectile_recycle_phase(frame);
    }

    fn run_iteration_phase(&mut self) {
        for (mut position, velocity) in self.movement.iter_mut(&mut self.world) {
            position.0 += velocity.0;
        }
        for (mut health, damage) in self.enemies.iter_mut(&mut self.world) {
            health.0 -= damage.0;
        }
        for (mut health, regen) in self.allies.iter_mut(&mut self.world) {
            health.0 += regen.0;
        }
        for mut lifetime in self.lifetimes.iter_mut(&mut self.world) {
            lifetime.0 = lifetime.0.saturating_sub(1);
            if lifetime.0 == 0 {
                lifetime.0 = 256;
            }
        }
    }

    fn run_ai_source_phase(&mut self, frame: &GameplayFrame) {
        self.target_entities.clear();
        self.target_slots.clear();
        for &slot in frame.ai_slots.iter() {
            let (target, mut cooldown) = self
                .ai
                .get_mut(&mut self.world, self.entities[slot])
                .expect("AI entity must have target and cooldown");
            let target_slot = target.0 as usize;
            self.target_entities.push(self.entities[target_slot]);
            self.target_slots.push(target_slot);
            cooldown.0 = cooldown.0.saturating_sub(1);
            self.cooldown_trace_checksum = gameplay_ai_trace_checksum(
                self.cooldown_trace_checksum,
                frame.index,
                slot,
                target_slot,
                cooldown.0,
            );
        }
    }

    fn run_target_position_phase(&mut self, frame: &GameplayFrame) {
        for ((&slot, &target), &target_slot) in frame
            .ai_slots
            .iter()
            .zip(&self.target_entities)
            .zip(&self.target_slots)
        {
            let position = self
                .positions
                .get_manual(&self.world, target)
                .expect("AI target must have PositionComponent");
            self.ai_lookup_checksum =
                gameplay_ai_lookup_checksum(self.ai_lookup_checksum, slot, target_slot, position);
        }
    }

    fn run_status_transition_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.remove_stunned.iter() {
            let mut entity = self.world.entity_mut(self.entities[slot]);
            assert!(entity.contains::<Stunned>(), "removing absent Stunned");
            entity.remove::<Stunned>();
        }
        for &slot in frame.add_stunned.iter() {
            let mut entity = self.world.entity_mut(self.entities[slot]);
            assert!(!entity.contains::<Stunned>(), "adding duplicate Stunned");
            entity.insert(Stunned);
        }
    }

    fn run_projectile_recycle_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.recycle_projectiles.iter() {
            assert!(self.world.despawn(self.entities[slot]));
            let generation = self.generations[slot].wrapping_add(1);
            self.generations[slot] = generation;
            self.entities[slot] = spawn_projectile(&mut self.world, slot, generation);
        }
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
        let mut component_mask_checksum = 0;
        let mut target_slot_checksum = 0;
        let mut owner_slot_checksum = 0;

        for (slot, &entity) in self.entities.iter().enumerate() {
            moving_count += usize::from(self.world.get::<VelocityComponent>(entity).is_some());
            health_count += usize::from(self.world.get::<Health>(entity).is_some());
            lifetime_count += usize::from(self.world.get::<Lifetime>(entity).is_some());
            stunned_count += usize::from(self.world.get::<Stunned>(entity).is_some());
            let mask = GAMEPLAY_MASK_POSITION
                | if self.world.get::<VelocityComponent>(entity).is_some() {
                    GAMEPLAY_MASK_VELOCITY
                } else {
                    0
                }
                | if self.world.get::<Health>(entity).is_some() {
                    GAMEPLAY_MASK_HEALTH
                } else {
                    0
                }
                | if self.world.get::<Damage>(entity).is_some() {
                    GAMEPLAY_MASK_DAMAGE
                } else {
                    0
                }
                | if self.world.get::<Regen>(entity).is_some() {
                    GAMEPLAY_MASK_REGEN
                } else {
                    0
                }
                | if self.world.get::<IsEnemy>(entity).is_some() {
                    GAMEPLAY_MASK_ENEMY
                } else {
                    0
                }
                | if self.world.get::<IsAlly>(entity).is_some() {
                    GAMEPLAY_MASK_ALLY
                } else {
                    0
                }
                | if self.world.get::<Lifetime>(entity).is_some() {
                    GAMEPLAY_MASK_LIFETIME
                } else {
                    0
                }
                | if self.world.get::<TargetSlot>(entity).is_some() {
                    GAMEPLAY_MASK_TARGET
                } else {
                    0
                }
                | if self.world.get::<Cooldown>(entity).is_some() {
                    GAMEPLAY_MASK_COOLDOWN
                } else {
                    0
                }
                | if self.world.get::<OwnerSlot>(entity).is_some() {
                    GAMEPLAY_MASK_OWNER
                } else {
                    0
                }
                | if self.world.get::<Stunned>(entity).is_some() {
                    GAMEPLAY_MASK_STUNNED
                } else {
                    0
                }
                | if self.world.get::<TagA>(entity).is_some() {
                    GAMEPLAY_MASK_TAG_A
                } else {
                    0
                }
                | if self.world.get::<TagB>(entity).is_some() {
                    GAMEPLAY_MASK_TAG_B
                } else {
                    0
                };
            component_mask_checksum =
                gameplay_mix_checksum(component_mask_checksum, slot as u64, mask as u64);

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
            if let Some(target) = self.world.get::<TargetSlot>(entity) {
                target_slot_checksum =
                    gameplay_mix_checksum(target_slot_checksum, slot as u64, target.0 as u64);
            }
            if let Some(owner) = self.world.get::<OwnerSlot>(entity) {
                owner_slot_checksum =
                    gameplay_mix_checksum(owner_slot_checksum, slot as u64, owner.0 as u64);
            }
            generation_checksum = gameplay_mix_checksum(
                generation_checksum,
                slot as u64,
                self.generations[slot] as u64,
            );
        }

        GameplayDigest {
            actual_entity_count: self
                .world
                .iter_entities()
                .filter(|entity| entity.contains::<PositionComponent>())
                .count(),
            unique_mapped_entity_count: self.entities.iter().copied().collect::<HashSet<_>>().len(),
            moving_count,
            health_count,
            lifetime_count,
            stunned_count,
            component_mask_checksum,
            position_checksum,
            health_checksum,
            lifetime_checksum,
            target_slot_checksum,
            owner_slot_checksum,
            cooldown_trace_checksum: self.cooldown_trace_checksum,
            generation_checksum,
            ai_lookup_checksum: self.ai_lookup_checksum,
        }
    }
}

impl GameplayPhaseAdapter for BevyGameplayWorld {
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
        BevyGameplayWorld::digest(self)
    }
}

fn insert_gameplay_components(
    world: &mut World,
    entity: Entity,
    values: GameplayEntityValues,
    slot: usize,
) {
    let mut entity = world.entity_mut(entity);
    entity.insert(values.position);
    if let Some(value) = values.velocity {
        entity.insert(value);
    }
    if let Some(value) = values.health {
        entity.insert(value);
    }
    if let Some(value) = values.damage {
        entity.insert(value);
    }
    if let Some(value) = values.regen {
        entity.insert(value);
    }
    if let Some(value) = values.enemy {
        entity.insert(value);
    }
    if let Some(value) = values.ally {
        entity.insert(value);
    }
    if let Some(value) = values.lifetime {
        entity.insert(value);
    }
    if let Some(value) = values.target {
        entity.insert(value);
    }
    if let Some(value) = values.cooldown {
        entity.insert(value);
    }
    if let Some(value) = values.owner {
        entity.insert(value);
    }
    if let Some(value) = values.stunned {
        entity.insert(value);
    }
    match gameplay_slot_spec(slot).variant {
        0 => {}
        1 => {
            entity.insert(TagA);
        }
        2 => {
            entity.insert(TagB);
        }
        3 => {
            entity.insert((TagA, TagB));
        }
        _ => unreachable!(),
    }
}

fn spawn_projectile(world: &mut World, slot: usize, generation: u32) -> Entity {
    let values = gameplay_entity_values(slot, generation, false);
    let position = values.position;
    let velocity = values.velocity.expect("projectile velocity");
    let damage = values.damage.expect("projectile damage");
    let lifetime = values.lifetime.expect("projectile lifetime");
    let owner = values.owner.expect("projectile owner");
    match gameplay_slot_spec(slot).variant {
        0 => world
            .spawn((position, velocity, damage, lifetime, owner))
            .id(),
        1 => world
            .spawn((position, velocity, damage, lifetime, owner, TagA))
            .id(),
        2 => world
            .spawn((position, velocity, damage, lifetime, owner, TagB))
            .id(),
        3 => world
            .spawn((position, velocity, damage, lifetime, owner, TagA, TagB))
            .id(),
        _ => unreachable!(),
    }
}

pub fn validate_gameplay_contract() {
    validate_gameplay_adapter(BevyGameplayWorld::new);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_full_gameplay_frames(
        group,
        "bevy",
        BevyGameplayWorld::new,
        BevyGameplayWorld::run_frame,
    );
}

pub fn bench_gameplay_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_gameplay_phases(group, "bevy", BevyGameplayWorld::new);
}

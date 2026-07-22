use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use shipyard::{EntitiesView, EntityId, Get, IntoIter, View, ViewMut, World};
use std::collections::HashSet;

pub(super) struct ShipyardGameplayWorld {
    world: World,
    entities: Vec<EntityId>,
    generations: Vec<u32>,
    target_entities: Vec<EntityId>,
    target_slots: Vec<usize>,
    ai_lookup_checksum: u64,
    cooldown_trace_checksum: u64,
}

impl ShipyardGameplayWorld {
    pub(super) fn new(trace: &GameplayTrace) -> Self {
        let mut world = World::new();
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }

        let mut entities = Vec::with_capacity(GAMEPLAY_ENTITY_COUNT);
        for (slot, &stunned) in initially_stunned.iter().enumerate() {
            let values = gameplay_entity_values(slot, 0, stunned);
            let entity = world.add_entity((values.position,));
            add_gameplay_components(&mut world, entity, values, slot);
            entities.push(entity);
        }

        Self {
            world,
            entities,
            generations: vec![0; GAMEPLAY_ENTITY_COUNT],
            target_entities: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            target_slots: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
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
        {
            let (mut positions, velocities) = self
                .world
                .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
                .unwrap();
            (&mut positions, &velocities)
                .iter()
                .for_each(|(position, velocity)| position.0 += velocity.0);
        }
        {
            let (mut health, damage) = self
                .world
                .borrow::<(ViewMut<Health>, View<Damage>)>()
                .unwrap();
            (&mut health, &damage)
                .iter()
                .for_each(|(health, damage)| health.0 -= damage.0);
        }
        {
            let (mut health, regen) = self
                .world
                .borrow::<(ViewMut<Health>, View<Regen>)>()
                .unwrap();
            (&mut health, &regen)
                .iter()
                .for_each(|(health, regen)| health.0 += regen.0);
        }
        {
            let mut lifetimes = self.world.borrow::<ViewMut<Lifetime>>().unwrap();
            (&mut lifetimes).iter().for_each(|lifetime| {
                lifetime.0 = lifetime.0.saturating_sub(1);
                if lifetime.0 == 0 {
                    lifetime.0 = 256;
                }
            });
        }
    }

    fn run_ai_source_phase(&mut self, frame: &GameplayFrame) {
        self.target_entities.clear();
        self.target_slots.clear();
        {
            let (targets, mut cooldowns) = self
                .world
                .borrow::<(View<TargetSlot>, ViewMut<Cooldown>)>()
                .unwrap();
            for &slot in frame.ai_slots.iter() {
                let entity = self.entities[slot];
                let target = (&targets)
                    .get(entity)
                    .expect("AI entity must have TargetSlot")
                    .0 as usize;
                let mut cooldown = (&mut cooldowns)
                    .get(entity)
                    .expect("AI entity must have Cooldown");
                cooldown.0 = cooldown.0.saturating_sub(1);
                self.target_entities.push(self.entities[target]);
                self.target_slots.push(target);
                self.cooldown_trace_checksum = gameplay_ai_trace_checksum(
                    self.cooldown_trace_checksum,
                    frame.index,
                    slot,
                    target,
                    cooldown.0,
                );
            }
        }
    }

    fn run_target_position_phase(&mut self, frame: &GameplayFrame) {
        {
            let positions = self.world.borrow::<View<PositionComponent>>().unwrap();
            for ((&slot, &target), &target_slot) in frame
                .ai_slots
                .iter()
                .zip(&self.target_entities)
                .zip(&self.target_slots)
            {
                let position = (&positions)
                    .get(target)
                    .expect("AI target must have PositionComponent");
                self.ai_lookup_checksum = gameplay_ai_lookup_checksum(
                    self.ai_lookup_checksum,
                    slot,
                    target_slot,
                    position,
                );
            }
        }
    }

    fn run_status_transition_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.remove_stunned.iter() {
            self.world
                .delete_component::<(Stunned,)>(self.entities[slot]);
        }
        for &slot in frame.add_stunned.iter() {
            self.world.add_component(self.entities[slot], (Stunned,));
        }
    }

    fn run_projectile_recycle_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.recycle_projectiles.iter() {
            assert!(self.world.delete_entity(self.entities[slot]));
            let generation = self.generations[slot].wrapping_add(1);
            self.generations[slot] = generation;
            self.entities[slot] = spawn_projectile(&mut self.world, slot, generation);
        }
    }

    pub(super) fn digest(&self) -> GameplayDigest {
        let (positions, velocities, healths, lifetimes, stunned) = self
            .world
            .borrow::<(
                View<PositionComponent>,
                View<VelocityComponent>,
                View<Health>,
                View<Lifetime>,
                View<Stunned>,
            )>()
            .unwrap();
        let damages = self.world.borrow::<View<Damage>>().unwrap();
        let regens = self.world.borrow::<View<Regen>>().unwrap();
        let enemies = self.world.borrow::<View<IsEnemy>>().unwrap();
        let allies = self.world.borrow::<View<IsAlly>>().unwrap();
        let targets = self.world.borrow::<View<TargetSlot>>().unwrap();
        let cooldowns = self.world.borrow::<View<Cooldown>>().unwrap();
        let owners = self.world.borrow::<View<OwnerSlot>>().unwrap();
        let tag_a = self.world.borrow::<View<TagA>>().unwrap();
        let tag_b = self.world.borrow::<View<TagB>>().unwrap();
        let entities_view = self.world.borrow::<EntitiesView>().unwrap();
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
            moving_count += usize::from((&velocities).get(entity).is_ok());
            health_count += usize::from((&healths).get(entity).is_ok());
            lifetime_count += usize::from((&lifetimes).get(entity).is_ok());
            stunned_count += usize::from((&stunned).get(entity).is_ok());
            let mask = GAMEPLAY_MASK_POSITION
                | if (&velocities).get(entity).is_ok() {
                    GAMEPLAY_MASK_VELOCITY
                } else {
                    0
                }
                | if (&healths).get(entity).is_ok() {
                    GAMEPLAY_MASK_HEALTH
                } else {
                    0
                }
                | if (&damages).get(entity).is_ok() {
                    GAMEPLAY_MASK_DAMAGE
                } else {
                    0
                }
                | if (&regens).get(entity).is_ok() {
                    GAMEPLAY_MASK_REGEN
                } else {
                    0
                }
                | if (&enemies).get(entity).is_ok() {
                    GAMEPLAY_MASK_ENEMY
                } else {
                    0
                }
                | if (&allies).get(entity).is_ok() {
                    GAMEPLAY_MASK_ALLY
                } else {
                    0
                }
                | if (&lifetimes).get(entity).is_ok() {
                    GAMEPLAY_MASK_LIFETIME
                } else {
                    0
                }
                | if (&targets).get(entity).is_ok() {
                    GAMEPLAY_MASK_TARGET
                } else {
                    0
                }
                | if (&cooldowns).get(entity).is_ok() {
                    GAMEPLAY_MASK_COOLDOWN
                } else {
                    0
                }
                | if (&owners).get(entity).is_ok() {
                    GAMEPLAY_MASK_OWNER
                } else {
                    0
                }
                | if (&stunned).get(entity).is_ok() {
                    GAMEPLAY_MASK_STUNNED
                } else {
                    0
                }
                | if (&tag_a).get(entity).is_ok() {
                    GAMEPLAY_MASK_TAG_A
                } else {
                    0
                }
                | if (&tag_b).get(entity).is_ok() {
                    GAMEPLAY_MASK_TAG_B
                } else {
                    0
                };
            component_mask_checksum =
                gameplay_mix_checksum(component_mask_checksum, slot as u64, mask as u64);

            let position = (&positions)
                .get(entity)
                .expect("every gameplay entity must have PositionComponent");
            position_checksum = gameplay_mix_checksum(
                position_checksum,
                slot as u64,
                (position.0.x.to_bits() as u64)
                    ^ ((position.0.y.to_bits() as u64) << 1)
                    ^ ((position.0.z.to_bits() as u64) << 2),
            );
            if let Ok(health) = (&healths).get(entity) {
                health_checksum =
                    gameplay_mix_checksum(health_checksum, slot as u64, health.0.to_bits() as u64);
            }
            if let Ok(lifetime) = (&lifetimes).get(entity) {
                lifetime_checksum =
                    gameplay_mix_checksum(lifetime_checksum, slot as u64, lifetime.0 as u64);
            }
            if let Ok(target) = (&targets).get(entity) {
                target_slot_checksum =
                    gameplay_mix_checksum(target_slot_checksum, slot as u64, target.0 as u64);
            }
            if let Ok(owner) = (&owners).get(entity) {
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
            actual_entity_count: entities_view.iter().count(),
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

impl GameplayPhaseAdapter for ShipyardGameplayWorld {
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
        ShipyardGameplayWorld::digest(self)
    }
}

fn add_gameplay_components(
    world: &mut World,
    entity: EntityId,
    values: GameplayEntityValues,
    slot: usize,
) {
    if let Some(value) = values.velocity {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.health {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.damage {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.regen {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.enemy {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.ally {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.lifetime {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.target {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.cooldown {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.owner {
        world.add_component(entity, (value,));
    }
    if let Some(value) = values.stunned {
        world.add_component(entity, (value,));
    }
    match gameplay_slot_spec(slot).variant {
        0 => {}
        1 => world.add_component(entity, (TagA,)),
        2 => world.add_component(entity, (TagB,)),
        3 => world.add_component(entity, (TagA, TagB)),
        _ => unreachable!(),
    }
}

fn spawn_projectile(world: &mut World, slot: usize, generation: u32) -> EntityId {
    let values = gameplay_entity_values(slot, generation, false);
    let position = values.position;
    let velocity = values.velocity.expect("projectile velocity");
    let damage = values.damage.expect("projectile damage");
    let lifetime = values.lifetime.expect("projectile lifetime");
    let owner = values.owner.expect("projectile owner");
    match gameplay_slot_spec(slot).variant {
        0 => world.add_entity((position, velocity, damage, lifetime, owner)),
        1 => world.add_entity((position, velocity, damage, lifetime, owner, TagA)),
        2 => world.add_entity((position, velocity, damage, lifetime, owner, TagB)),
        3 => world.add_entity((position, velocity, damage, lifetime, owner, TagA, TagB)),
        _ => unreachable!(),
    }
}

pub fn validate_gameplay_contract() {
    validate_gameplay_adapter(ShipyardGameplayWorld::new);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_full_gameplay_frames(
        group,
        "shipyard",
        ShipyardGameplayWorld::new,
        ShipyardGameplayWorld::run_frame,
    );
}

pub fn bench_gameplay_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_gameplay_phases(group, "shipyard", ShipyardGameplayWorld::new);
}

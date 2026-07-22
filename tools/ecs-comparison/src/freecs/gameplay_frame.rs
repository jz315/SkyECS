use super::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::collections::{BTreeMap, HashSet};

const GAMEPLAY_MOVE_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
const GAMEPLAY_ENEMY_MASK: u64 = HEALTH_MASK | DAMAGE_MASK | IS_ENEMY_MASK;
const GAMEPLAY_ALLY_MASK: u64 = HEALTH_MASK | REGEN_MASK | IS_ALLY_MASK;

pub(super) struct FreecsGameplayWorld {
    world: World,
    entities: Vec<Entity>,
    generations: Vec<u32>,
    target_entities: Vec<Entity>,
    target_slots: Vec<usize>,
    ai_lookup_checksum: u64,
    cooldown_trace_checksum: u64,
}

impl FreecsGameplayWorld {
    pub(super) fn new(trace: &GameplayTrace) -> Self {
        let mut world = World::default();
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }

        let mut groups = BTreeMap::<u64, Vec<usize>>::new();
        for (slot, &stunned) in initially_stunned.iter().enumerate() {
            let values = gameplay_entity_values(slot, 0, stunned);
            groups
                .entry(gameplay_mask(&values, slot))
                .or_default()
                .push(slot);
        }

        let mut entities = vec![None; GAMEPLAY_ENTITY_COUNT];
        for (mask, slots) in groups {
            let mut slot_values = slots.iter().copied();
            let spawned = world.spawn_batch(mask, slots.len(), |table, index| {
                let slot = slot_values
                    .next()
                    .expect("initializer count must match batch");
                write_gameplay_values(
                    table,
                    index,
                    gameplay_entity_values(slot, 0, initially_stunned[slot]),
                    slot,
                );
            });
            for (slot, entity) in slots.into_iter().zip(spawned) {
                entities[slot] = Some(entity);
            }
        }

        for mask in [
            GAMEPLAY_MOVE_MASK,
            GAMEPLAY_ENEMY_MASK,
            GAMEPLAY_ALLY_MASK,
            LIFETIME_MASK,
        ] {
            world.for_each_mut(mask, 0, |_entity, _table, _index| {});
        }

        Self {
            world,
            entities: entities
                .into_iter()
                .map(|entity| entity.expect("every logical slot must be spawned"))
                .collect(),
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
        self.world
            .for_each_mut(GAMEPLAY_MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
        self.world
            .for_each_mut(GAMEPLAY_ENEMY_MASK, 0, |_entity, table, index| {
                table.health[index].0 -= table.damage[index].0;
            });
        self.world
            .for_each_mut(GAMEPLAY_ALLY_MASK, 0, |_entity, table, index| {
                table.health[index].0 += table.regen[index].0;
            });
        self.world
            .for_each_mut(LIFETIME_MASK, 0, |_entity, table, index| {
                table.lifetime[index].0 = table.lifetime[index].0.saturating_sub(1);
                if table.lifetime[index].0 == 0 && table.mask & VELOCITY_MASK == 0 {
                    table.lifetime[index].0 = 256;
                }
            });
    }

    fn run_ai_source_phase(&mut self, frame: &GameplayFrame) {
        self.target_entities.clear();
        self.target_slots.clear();
        for &slot in frame.ai_slots.iter() {
            let ai = self.entities[slot];
            let target = self
                .world
                .get_target_slot(ai)
                .expect("AI entity must have TargetSlot")
                .0 as usize;
            let cooldown = self
                .world
                .get_cooldown_mut(ai)
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

    fn run_target_position_phase(&mut self, frame: &GameplayFrame) {
        for ((&slot, &target), &target_slot) in frame
            .ai_slots
            .iter()
            .zip(&self.target_entities)
            .zip(&self.target_slots)
        {
            let position = self
                .world
                .get_position(target)
                .expect("AI target must have PositionComponent");
            self.ai_lookup_checksum =
                gameplay_ai_lookup_checksum(self.ai_lookup_checksum, slot, target_slot, position);
        }
    }

    fn run_status_transition_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.remove_stunned.iter() {
            assert!(self.world.remove_stunned(self.entities[slot]));
        }
        for &slot in frame.add_stunned.iter() {
            assert!(self.world.get_stunned(self.entities[slot]).is_none());
            self.world.set_stunned(self.entities[slot], Stunned);
        }
    }

    fn run_projectile_recycle_phase(&mut self, frame: &GameplayFrame) {
        let old_entities = frame
            .recycle_projectiles
            .iter()
            .map(|&slot| self.entities[slot])
            .collect::<Vec<_>>();
        assert_eq!(
            self.world.despawn_entities(&old_entities).len(),
            GAMEPLAY_PROJECTILE_RECYCLES_PER_FRAME
        );
        for &slot in frame.recycle_projectiles.iter() {
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
        let mut value_checksums = GameplayValueChecksums::default();

        for (slot, &entity) in self.entities.iter().enumerate() {
            moving_count += usize::from(self.world.get_velocity(entity).is_some());
            health_count += usize::from(self.world.get_health(entity).is_some());
            lifetime_count += usize::from(self.world.get_lifetime(entity).is_some());
            stunned_count += usize::from(self.world.get_stunned(entity).is_some());
            let mask = GAMEPLAY_MASK_POSITION
                | if self.world.get_velocity(entity).is_some() {
                    GAMEPLAY_MASK_VELOCITY
                } else {
                    0
                }
                | if self.world.get_health(entity).is_some() {
                    GAMEPLAY_MASK_HEALTH
                } else {
                    0
                }
                | if self.world.get_damage(entity).is_some() {
                    GAMEPLAY_MASK_DAMAGE
                } else {
                    0
                }
                | if self.world.get_regen(entity).is_some() {
                    GAMEPLAY_MASK_REGEN
                } else {
                    0
                }
                | if self.world.get_is_enemy(entity).is_some() {
                    GAMEPLAY_MASK_ENEMY
                } else {
                    0
                }
                | if self.world.get_is_ally(entity).is_some() {
                    GAMEPLAY_MASK_ALLY
                } else {
                    0
                }
                | if self.world.get_lifetime(entity).is_some() {
                    GAMEPLAY_MASK_LIFETIME
                } else {
                    0
                }
                | if self.world.get_target_slot(entity).is_some() {
                    GAMEPLAY_MASK_TARGET
                } else {
                    0
                }
                | if self.world.get_cooldown(entity).is_some() {
                    GAMEPLAY_MASK_COOLDOWN
                } else {
                    0
                }
                | if self.world.get_owner_slot(entity).is_some() {
                    GAMEPLAY_MASK_OWNER
                } else {
                    0
                }
                | if self.world.get_stunned(entity).is_some() {
                    GAMEPLAY_MASK_STUNNED
                } else {
                    0
                }
                | if self.world.get_tag_a(entity).is_some() {
                    GAMEPLAY_MASK_TAG_A
                } else {
                    0
                }
                | if self.world.get_tag_b(entity).is_some() {
                    GAMEPLAY_MASK_TAG_B
                } else {
                    0
                };
            component_mask_checksum =
                gameplay_mix_checksum(component_mask_checksum, slot as u64, mask as u64);
            value_checksums.observe(
                slot,
                self.world.get_velocity(entity),
                self.world.get_damage(entity),
                self.world.get_regen(entity),
                self.world.get_cooldown(entity),
            );
            let position = self
                .world
                .get_position(entity)
                .expect("every gameplay entity must have PositionComponent");
            position_checksum = gameplay_mix_checksum(
                position_checksum,
                slot as u64,
                (position.0.x.to_bits() as u64)
                    ^ ((position.0.y.to_bits() as u64) << 1)
                    ^ ((position.0.z.to_bits() as u64) << 2),
            );
            if let Some(health) = self.world.get_health(entity) {
                health_checksum =
                    gameplay_mix_checksum(health_checksum, slot as u64, health.0.to_bits() as u64);
            }
            if let Some(lifetime) = self.world.get_lifetime(entity) {
                lifetime_checksum =
                    gameplay_mix_checksum(lifetime_checksum, slot as u64, lifetime.0 as u64);
            }
            if let Some(target) = self.world.get_target_slot(entity) {
                target_slot_checksum =
                    gameplay_mix_checksum(target_slot_checksum, slot as u64, target.0 as u64);
            }
            if let Some(owner) = self.world.get_owner_slot(entity) {
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
            actual_entity_count: self.world.entity_count(),
            unique_mapped_entity_count: self.entities.iter().copied().collect::<HashSet<_>>().len(),
            moving_count,
            health_count,
            lifetime_count,
            stunned_count,
            component_mask_checksum,
            position_checksum,
            velocity_checksum: value_checksums.velocity,
            health_checksum,
            damage_checksum: value_checksums.damage,
            regen_checksum: value_checksums.regen,
            cooldown_checksum: value_checksums.cooldown,
            lifetime_checksum,
            target_slot_checksum,
            owner_slot_checksum,
            cooldown_trace_checksum: self.cooldown_trace_checksum,
            generation_checksum,
            ai_lookup_checksum: self.ai_lookup_checksum,
        }
    }
}

impl GameplayPhaseAdapter for FreecsGameplayWorld {
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
        FreecsGameplayWorld::digest(self)
    }
}

fn write_gameplay_values(
    table: &mut ComponentArrays,
    index: usize,
    values: GameplayEntityValues,
    slot: usize,
) {
    table.position[index] = values.position;
    if let Some(value) = values.velocity {
        table.velocity[index] = value;
    }
    if let Some(value) = values.health {
        table.health[index] = value;
    }
    if let Some(value) = values.damage {
        table.damage[index] = value;
    }
    if let Some(value) = values.regen {
        table.regen[index] = value;
    }
    if let Some(value) = values.enemy {
        table.is_enemy[index] = value;
    }
    if let Some(value) = values.ally {
        table.is_ally[index] = value;
    }
    if let Some(value) = values.lifetime {
        table.lifetime[index] = value;
    }
    if let Some(value) = values.target {
        table.target_slot[index] = value;
    }
    if let Some(value) = values.cooldown {
        table.cooldown[index] = value;
    }
    if let Some(value) = values.owner {
        table.owner_slot[index] = value;
    }
    if let Some(value) = values.stunned {
        table.stunned[index] = value;
    }
    match gameplay_slot_spec(slot).variant {
        0 => {}
        1 => table.tag_a[index] = TagA,
        2 => table.tag_b[index] = TagB,
        3 => {
            table.tag_a[index] = TagA;
            table.tag_b[index] = TagB;
        }
        _ => unreachable!(),
    }
}

fn gameplay_mask(values: &GameplayEntityValues, slot: usize) -> u64 {
    let mut mask = POSITION_MASK;
    if values.velocity.is_some() {
        mask |= VELOCITY_MASK;
    }
    if values.health.is_some() {
        mask |= HEALTH_MASK;
    }
    if values.damage.is_some() {
        mask |= DAMAGE_MASK;
    }
    if values.regen.is_some() {
        mask |= REGEN_MASK;
    }
    if values.enemy.is_some() {
        mask |= IS_ENEMY_MASK;
    }
    if values.ally.is_some() {
        mask |= IS_ALLY_MASK;
    }
    if values.lifetime.is_some() {
        mask |= LIFETIME_MASK;
    }
    if values.target.is_some() {
        mask |= TARGET_SLOT_MASK;
    }
    if values.cooldown.is_some() {
        mask |= COOLDOWN_MASK;
    }
    if values.owner.is_some() {
        mask |= OWNER_SLOT_MASK;
    }
    if values.stunned.is_some() {
        mask |= STUNNED_MASK;
    }
    mask | match gameplay_slot_spec(slot).variant {
        0 => 0,
        1 => TAG_A_MASK,
        2 => TAG_B_MASK,
        3 => TAG_A_MASK | TAG_B_MASK,
        _ => unreachable!(),
    }
}

fn spawn_projectile(world: &mut World, slot: usize, generation: u32) -> Entity {
    let values = gameplay_entity_values(slot, generation, false);
    let mask = gameplay_mask(&values, slot);
    world
        .spawn_batch(mask, 1, |table, index| {
            write_gameplay_values(table, index, values, slot);
        })
        .pop()
        .expect("single projectile spawn must return one entity")
}

pub fn validate_gameplay_contract() {
    validate_gameplay_adapter(FreecsGameplayWorld::new);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_full_gameplay_frames(
        group,
        "freecs",
        FreecsGameplayWorld::new,
        FreecsGameplayWorld::run_frame,
    );
}

pub fn bench_gameplay_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    crate::common::bench_gameplay_phases(group, "freecs", FreecsGameplayWorld::new);
}

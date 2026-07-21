use super::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::collections::BTreeMap;
use std::hint::black_box;

const GAMEPLAY_MOVE_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
const GAMEPLAY_ENEMY_MASK: u64 = HEALTH_MASK | DAMAGE_MASK | IS_ENEMY_MASK;
const GAMEPLAY_ALLY_MASK: u64 = HEALTH_MASK | REGEN_MASK | IS_ALLY_MASK;

pub(super) struct FreecsGameplayWorld {
    world: World,
    entities: Vec<Entity>,
    generations: Vec<u32>,
    target_entities: Vec<Entity>,
    ai_lookup_checksum: u64,
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
            ai_lookup_checksum: 0,
        }
    }

    pub(super) fn run_frame(&mut self, frame: &GameplayFrame) {
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
                if table.lifetime[index].0 == 0 {
                    table.lifetime[index].0 = 256;
                }
            });

        self.target_entities.clear();
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
        }
        for (&slot, &target) in frame.ai_slots.iter().zip(&self.target_entities) {
            let position = self
                .world
                .get_position(target)
                .expect("AI target must have PositionComponent");
            self.ai_lookup_checksum = gameplay_mix_checksum(
                self.ai_lookup_checksum,
                slot as u64,
                position.0.x.to_bits() as u64,
            );
        }

        for &slot in frame.remove_stunned.iter() {
            assert!(self.world.remove_stunned(self.entities[slot]));
        }
        for &slot in frame.add_stunned.iter() {
            assert!(self.world.get_stunned(self.entities[slot]).is_none());
            self.world.set_stunned(self.entities[slot], Stunned);
        }

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

        for (slot, &entity) in self.entities.iter().enumerate() {
            moving_count += usize::from(self.world.get_velocity(entity).is_some());
            health_count += usize::from(self.world.get_health(entity).is_some());
            lifetime_count += usize::from(self.world.get_lifetime(entity).is_some());
            stunned_count += usize::from(self.world.get_stunned(entity).is_some());
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
            generation_checksum = gameplay_mix_checksum(
                generation_checksum,
                slot as u64,
                self.generations[slot] as u64,
            );
        }

        GameplayDigest {
            entity_count: self.entities.len(),
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
    let trace = GameplayTrace::standard();
    let mut gameplay = FreecsGameplayWorld::new(&trace);
    for frame in trace.frames() {
        gameplay.run_frame(frame);
    }
    assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/freecs", |bencher| {
        let trace = GameplayTrace::standard();
        let mut gameplay = FreecsGameplayWorld::new(&trace);
        let mut frame = 0;
        bencher.iter(|| {
            gameplay.run_frame(&trace.frames()[frame]);
            frame = (frame + 1) % GAMEPLAY_FRAME_COUNT;
            black_box(&gameplay.world);
        });
    });
}

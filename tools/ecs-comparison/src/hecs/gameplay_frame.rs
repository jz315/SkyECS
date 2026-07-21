use crate::common::*;
use criterion::{measurement::WallTime, BenchmarkGroup};
use hecs::{Entity, EntityBuilder, PreparedQuery, World};
use std::hint::black_box;

pub(super) struct HecsGameplayWorld {
    world: World,
    entities: Vec<Entity>,
    generations: Vec<u32>,
    target_entities: Vec<Entity>,
    movement: PreparedQuery<(&'static mut PositionComponent, &'static VelocityComponent)>,
    enemies: PreparedQuery<(&'static mut Health, &'static Damage)>,
    allies: PreparedQuery<(&'static mut Health, &'static Regen)>,
    lifetimes: PreparedQuery<&'static mut Lifetime>,
    ai: PreparedQuery<(&'static TargetSlot, &'static mut Cooldown)>,
    positions: PreparedQuery<&'static PositionComponent>,
    ai_lookup_checksum: u64,
}

impl HecsGameplayWorld {
    pub(super) fn new(trace: &GameplayTrace) -> Self {
        let mut world = World::new();
        let mut builder = EntityBuilder::new();
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }

        let mut entities = Vec::with_capacity(GAMEPLAY_ENTITY_COUNT);
        for (slot, &stunned) in initially_stunned.iter().enumerate() {
            add_gameplay_components(&mut builder, gameplay_entity_values(slot, 0, stunned), slot);
            entities.push(world.spawn(builder.build()));
        }

        Self {
            world,
            entities,
            generations: vec![0; GAMEPLAY_ENTITY_COUNT],
            target_entities: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            movement: PreparedQuery::default(),
            enemies: PreparedQuery::default(),
            allies: PreparedQuery::default(),
            lifetimes: PreparedQuery::default(),
            ai: PreparedQuery::default(),
            positions: PreparedQuery::default(),
            ai_lookup_checksum: 0,
        }
    }

    pub(super) fn run_frame(&mut self, frame: &GameplayFrame) {
        for (position, velocity) in self.movement.query(&self.world).iter() {
            position.0 += velocity.0;
        }
        for (health, damage) in self.enemies.query(&self.world).iter() {
            health.0 -= damage.0;
        }
        for (health, regen) in self.allies.query(&self.world).iter() {
            health.0 += regen.0;
        }
        for lifetime in self.lifetimes.query(&self.world).iter() {
            lifetime.0 = lifetime.0.saturating_sub(1);
            if lifetime.0 == 0 {
                lifetime.0 = 256;
            }
        }

        self.target_entities.clear();
        {
            let mut ai = self.ai.view_mut(&mut self.world);
            for &slot in frame.ai_slots.iter() {
                let (target, cooldown) = ai
                    .get_mut(self.entities[slot])
                    .expect("AI entity must have target and cooldown");
                self.target_entities.push(self.entities[target.0 as usize]);
                cooldown.0 = cooldown.0.saturating_sub(1);
            }
        }
        {
            let positions = self.positions.view_mut(&mut self.world);
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

        for &slot in frame.remove_stunned.iter() {
            self.world
                .remove_one::<Stunned>(self.entities[slot])
                .expect("removing absent Stunned");
        }
        for &slot in frame.add_stunned.iter() {
            self.world
                .insert_one(self.entities[slot], Stunned)
                .expect("adding duplicate Stunned");
        }

        for &slot in frame.recycle_projectiles.iter() {
            self.world
                .despawn(self.entities[slot])
                .expect("projectile must be alive");
            let generation = self.generations[slot].wrapping_add(1);
            self.generations[slot] = generation;
            self.entities[slot] = spawn_projectile(&mut self.world, slot, generation);
        }
        debug_assert_eq!(self.world.len(), GAMEPLAY_ENTITY_COUNT as u32);
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
            moving_count += usize::from(self.world.get::<&VelocityComponent>(entity).is_ok());
            health_count += usize::from(self.world.get::<&Health>(entity).is_ok());
            lifetime_count += usize::from(self.world.get::<&Lifetime>(entity).is_ok());
            stunned_count += usize::from(self.world.get::<&Stunned>(entity).is_ok());

            let position = self
                .world
                .get::<&PositionComponent>(entity)
                .expect("every gameplay entity must have PositionComponent");
            position_checksum = gameplay_mix_checksum(
                position_checksum,
                slot as u64,
                (position.0.x.to_bits() as u64)
                    ^ ((position.0.y.to_bits() as u64) << 1)
                    ^ ((position.0.z.to_bits() as u64) << 2),
            );
            if let Ok(health) = self.world.get::<&Health>(entity) {
                health_checksum =
                    gameplay_mix_checksum(health_checksum, slot as u64, health.0.to_bits() as u64);
            }
            if let Ok(lifetime) = self.world.get::<&Lifetime>(entity) {
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
            entity_count: self.world.len() as usize,
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

fn add_gameplay_components(builder: &mut EntityBuilder, values: GameplayEntityValues, slot: usize) {
    builder.add(values.position);
    if let Some(value) = values.velocity {
        builder.add(value);
    }
    if let Some(value) = values.health {
        builder.add(value);
    }
    if let Some(value) = values.damage {
        builder.add(value);
    }
    if let Some(value) = values.regen {
        builder.add(value);
    }
    if let Some(value) = values.enemy {
        builder.add(value);
    }
    if let Some(value) = values.ally {
        builder.add(value);
    }
    if let Some(value) = values.lifetime {
        builder.add(value);
    }
    if let Some(value) = values.target {
        builder.add(value);
    }
    if let Some(value) = values.cooldown {
        builder.add(value);
    }
    if let Some(value) = values.owner {
        builder.add(value);
    }
    if let Some(value) = values.stunned {
        builder.add(value);
    }
    match gameplay_slot_spec(slot).variant {
        0 => {}
        1 => {
            builder.add(TagA);
        }
        2 => {
            builder.add(TagB);
        }
        3 => {
            builder.add(TagA).add(TagB);
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
        0 => world.spawn((position, velocity, damage, lifetime, owner)),
        1 => world.spawn((position, velocity, damage, lifetime, owner, TagA)),
        2 => world.spawn((position, velocity, damage, lifetime, owner, TagB)),
        3 => world.spawn((position, velocity, damage, lifetime, owner, TagA, TagB)),
        _ => unreachable!(),
    }
}

pub fn validate_gameplay_contract() {
    let trace = GameplayTrace::standard();
    let mut gameplay = HecsGameplayWorld::new(&trace);
    for frame in trace.frames() {
        gameplay.run_frame(frame);
    }
    assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
}

pub fn bench_gameplay_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("frame/hecs", |bencher| {
        let trace = GameplayTrace::standard();
        let mut gameplay = HecsGameplayWorld::new(&trace);
        let mut frame = 0;
        bencher.iter(|| {
            gameplay.run_frame(&trace.frames()[frame]);
            frame = (frame + 1) % GAMEPLAY_FRAME_COUNT;
            black_box(&gameplay.world);
        });
    });
}

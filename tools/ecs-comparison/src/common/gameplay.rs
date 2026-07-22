use super::{
    Cooldown, Damage, Health, IsAlly, IsEnemy, Lifetime, OwnerSlot, PositionComponent, Regen,
    Stunned, TargetSlot, VelocityComponent,
};
use cgmath::Vector3;

pub const GAMEPLAY_ENTITY_COUNT: usize = 65_536;
pub const GAMEPLAY_FRAME_COUNT: usize = 256;

pub const GAMEPLAY_MOVER_START: usize = 0;
pub const GAMEPLAY_MOVER_COUNT: usize = 20_480;
pub const GAMEPLAY_COMBAT_START: usize = GAMEPLAY_MOVER_START + GAMEPLAY_MOVER_COUNT;
pub const GAMEPLAY_COMBAT_COUNT: usize = 16_384;
pub const GAMEPLAY_ENEMY_COUNT: usize = GAMEPLAY_COMBAT_COUNT / 2;
pub const GAMEPLAY_AI_START: usize = GAMEPLAY_COMBAT_START + GAMEPLAY_COMBAT_COUNT;
pub const GAMEPLAY_AI_COUNT: usize = 8_192;
pub const GAMEPLAY_PROJECTILE_START: usize = GAMEPLAY_AI_START + GAMEPLAY_AI_COUNT;
pub const GAMEPLAY_PROJECTILE_COUNT: usize = 8_192;
pub const GAMEPLAY_STATIC_START: usize = GAMEPLAY_PROJECTILE_START + GAMEPLAY_PROJECTILE_COUNT;
pub const GAMEPLAY_STATIC_COUNT: usize = 8_192;
pub const GAMEPLAY_EFFECT_START: usize = GAMEPLAY_STATIC_START + GAMEPLAY_STATIC_COUNT;
pub const GAMEPLAY_EFFECT_COUNT: usize = 4_096;

pub const GAMEPLAY_MOVING_COUNT: usize =
    GAMEPLAY_MOVER_COUNT + GAMEPLAY_COMBAT_COUNT + GAMEPLAY_AI_COUNT + GAMEPLAY_PROJECTILE_COUNT;
pub const GAMEPLAY_HEALTH_COUNT: usize = GAMEPLAY_COMBAT_COUNT + GAMEPLAY_AI_COUNT;
pub const GAMEPLAY_LIFETIME_COUNT: usize = GAMEPLAY_PROJECTILE_COUNT + GAMEPLAY_EFFECT_COUNT;

pub const GAMEPLAY_AI_LOOKUPS_PER_FRAME: usize = 2_048;
pub const GAMEPLAY_STATUS_CHANGES_PER_FRAME: usize = 128;
pub const GAMEPLAY_PROJECTILE_RECYCLES_PER_FRAME: usize = 128;
pub const GAMEPLAY_STATUS_DURATION_FRAMES: usize = 8;
pub const GAMEPLAY_PROJECTILE_LIFETIME_FRAMES: usize = 64;
pub const GAMEPLAY_STUNNED_COUNT: usize =
    GAMEPLAY_STATUS_CHANGES_PER_FRAME * GAMEPLAY_STATUS_DURATION_FRAMES;
pub const GAMEPLAY_CONTRACT_CHECKPOINTS: [usize; 10] = [0, 1, 7, 8, 15, 16, 63, 64, 127, 255];

pub const GAMEPLAY_MASK_POSITION: u16 = 1 << 0;
pub const GAMEPLAY_MASK_VELOCITY: u16 = 1 << 1;
pub const GAMEPLAY_MASK_HEALTH: u16 = 1 << 2;
pub const GAMEPLAY_MASK_DAMAGE: u16 = 1 << 3;
pub const GAMEPLAY_MASK_REGEN: u16 = 1 << 4;
pub const GAMEPLAY_MASK_ENEMY: u16 = 1 << 5;
pub const GAMEPLAY_MASK_ALLY: u16 = 1 << 6;
pub const GAMEPLAY_MASK_LIFETIME: u16 = 1 << 7;
pub const GAMEPLAY_MASK_TARGET: u16 = 1 << 8;
pub const GAMEPLAY_MASK_COOLDOWN: u16 = 1 << 9;
pub const GAMEPLAY_MASK_OWNER: u16 = 1 << 10;
pub const GAMEPLAY_MASK_STUNNED: u16 = 1 << 11;
pub const GAMEPLAY_MASK_TAG_A: u16 = 1 << 12;
pub const GAMEPLAY_MASK_TAG_B: u16 = 1 << 13;

const GAMEPLAY_VARIANT_COUNT: usize = 4;
const GAMEPLAY_STATUS_COHORT_COUNT: usize = GAMEPLAY_STATUS_DURATION_FRAMES * 2;
const GAMEPLAY_AI_COHORT_COUNT: usize = GAMEPLAY_AI_COUNT / GAMEPLAY_AI_LOOKUPS_PER_FRAME;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameplayKind {
    Mover,
    Enemy,
    Ally,
    Ai,
    Projectile,
    Static,
    Effect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameplaySlotSpec {
    pub slot: usize,
    pub kind: GameplayKind,
    /// Two low-cardinality tags turn the seven semantic layouts into 32+
    /// stable archetypes without changing the systems that match them.
    pub variant: u8,
}

#[derive(Clone, Copy)]
pub struct GameplayEntityValues {
    pub position: PositionComponent,
    pub velocity: Option<VelocityComponent>,
    pub health: Option<Health>,
    pub damage: Option<Damage>,
    pub regen: Option<Regen>,
    pub enemy: Option<IsEnemy>,
    pub ally: Option<IsAlly>,
    pub lifetime: Option<Lifetime>,
    pub target: Option<TargetSlot>,
    pub cooldown: Option<Cooldown>,
    pub owner: Option<OwnerSlot>,
    pub stunned: Option<Stunned>,
}

#[derive(Clone, Debug)]
pub struct GameplayFrame {
    pub index: usize,
    /// One quarter of the AI population. Each selected AI performs a direct
    /// lookup of its target's Position component.
    pub ai_slots: Box<[usize]>,
    pub remove_stunned: Box<[usize]>,
    pub add_stunned: Box<[usize]>,
    /// Existing projectiles are despawned and replacements are spawned into
    /// the same logical slots. The adapter must refresh its slot->entity map.
    pub recycle_projectiles: Box<[usize]>,
}

#[derive(Clone, Debug)]
pub struct GameplayTrace {
    frames: Box<[GameplayFrame]>,
    initially_stunned: Box<[usize]>,
}

impl GameplayTrace {
    pub fn standard() -> Self {
        let initially_stunned = (GAMEPLAY_STATUS_DURATION_FRAMES..GAMEPLAY_STATUS_COHORT_COUNT)
            .flat_map(status_cohort)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let frames = (0..GAMEPLAY_FRAME_COUNT)
            .map(|index| GameplayFrame {
                index,
                ai_slots: ai_cohort(index % GAMEPLAY_AI_COHORT_COUNT)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                remove_stunned: status_cohort(
                    (index + GAMEPLAY_STATUS_DURATION_FRAMES) % GAMEPLAY_STATUS_COHORT_COUNT,
                )
                .collect::<Vec<_>>()
                .into_boxed_slice(),
                add_stunned: status_cohort(index % GAMEPLAY_STATUS_COHORT_COUNT)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                recycle_projectiles: projectile_cohort(index % GAMEPLAY_PROJECTILE_LIFETIME_FRAMES)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            frames,
            initially_stunned,
        }
    }

    pub fn frames(&self) -> &[GameplayFrame] {
        &self.frames
    }

    pub fn initially_stunned(&self) -> &[usize] {
        &self.initially_stunned
    }
}

impl Default for GameplayTrace {
    fn default() -> Self {
        Self::standard()
    }
}

pub fn gameplay_slot_spec(slot: usize) -> GameplaySlotSpec {
    assert!(slot < GAMEPLAY_ENTITY_COUNT, "gameplay slot out of range");
    let kind = if slot < GAMEPLAY_COMBAT_START {
        GameplayKind::Mover
    } else if slot < GAMEPLAY_COMBAT_START + GAMEPLAY_ENEMY_COUNT {
        GameplayKind::Enemy
    } else if slot < GAMEPLAY_AI_START {
        GameplayKind::Ally
    } else if slot < GAMEPLAY_PROJECTILE_START {
        GameplayKind::Ai
    } else if slot < GAMEPLAY_STATIC_START {
        GameplayKind::Projectile
    } else if slot < GAMEPLAY_EFFECT_START {
        GameplayKind::Static
    } else {
        GameplayKind::Effect
    };
    GameplaySlotSpec {
        slot,
        kind,
        variant: (slot % GAMEPLAY_VARIANT_COUNT) as u8,
    }
}

pub fn gameplay_entity_values(slot: usize, generation: u32, stunned: bool) -> GameplayEntityValues {
    let spec = gameplay_slot_spec(slot);
    let position = gameplay_position(slot, generation);
    let (velocity, health, damage, regen, enemy, ally, lifetime, target, cooldown, owner) =
        match spec.kind {
            GameplayKind::Mover => (
                Some(VelocityComponent(Vector3::new(1.0, 0.5, 0.25))),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            GameplayKind::Enemy => (
                Some(VelocityComponent(Vector3::new(0.25, 1.0, 0.0))),
                Some(Health(100.0)),
                Some(Damage(0.75)),
                None,
                Some(IsEnemy),
                None,
                None,
                None,
                None,
                None,
            ),
            GameplayKind::Ally => (
                Some(VelocityComponent(Vector3::new(0.0, 0.75, 0.25))),
                Some(Health(60.0)),
                None,
                Some(Regen(0.25)),
                None,
                Some(IsAlly),
                None,
                None,
                None,
                None,
            ),
            GameplayKind::Ai => (
                Some(VelocityComponent(Vector3::new(0.125, 0.25, 0.0))),
                Some(Health(80.0)),
                None,
                None,
                None,
                None,
                None,
                Some(TargetSlot(gameplay_ai_target_slot(slot) as u32)),
                Some(Cooldown(((slot - GAMEPLAY_AI_START) % 32) as u32)),
                None,
            ),
            GameplayKind::Projectile => (
                Some(VelocityComponent(Vector3::new(2.0, 0.0, 0.0))),
                None,
                Some(Damage(1.0)),
                None,
                None,
                None,
                Some(Lifetime(GAMEPLAY_PROJECTILE_LIFETIME_FRAMES as u32)),
                None,
                None,
                Some(OwnerSlot(
                    (GAMEPLAY_COMBAT_START
                        + (slot - GAMEPLAY_PROJECTILE_START) % GAMEPLAY_COMBAT_COUNT)
                        as u32,
                )),
            ),
            GameplayKind::Static => (None, None, None, None, None, None, None, None, None, None),
            GameplayKind::Effect => (
                None,
                None,
                None,
                None,
                None,
                None,
                Some(Lifetime(256)),
                None,
                None,
                None,
            ),
        };

    GameplayEntityValues {
        position,
        velocity,
        health,
        damage,
        regen,
        enemy,
        ally,
        lifetime,
        target,
        cooldown,
        owner,
        stunned: stunned.then_some(Stunned),
    }
}

pub fn gameplay_ai_target_slot(ai_slot: usize) -> usize {
    debug_assert!((GAMEPLAY_AI_START..GAMEPLAY_PROJECTILE_START).contains(&ai_slot));
    // Keep targets outside the projectile range so lifecycle replacement does
    // not turn the lookup phase into a stale-handle special case.
    let targetable_count = GAMEPLAY_PROJECTILE_START;
    ai_slot.wrapping_mul(1_103_515_245).wrapping_add(12_345) % targetable_count
}

fn gameplay_position(slot: usize, generation: u32) -> PositionComponent {
    let x = (slot & 0xff) as f32 * 0.25 + generation as f32 * 0.5;
    let y = ((slot >> 8) & 0xff) as f32 * 0.125;
    let z = (slot >> 16) as f32 * 0.5;
    PositionComponent(Vector3::new(x, y, z))
}

fn ai_cohort(cohort: usize) -> impl Iterator<Item = usize> {
    (0..GAMEPLAY_AI_LOOKUPS_PER_FRAME)
        .map(move |index| GAMEPLAY_AI_START + cohort + index * GAMEPLAY_AI_COHORT_COUNT)
}

fn status_cohort(cohort: usize) -> impl Iterator<Item = usize> {
    (0..GAMEPLAY_STATUS_CHANGES_PER_FRAME)
        .map(move |index| GAMEPLAY_COMBAT_START + cohort + index * GAMEPLAY_STATUS_COHORT_COUNT)
}

fn projectile_cohort(cohort: usize) -> impl Iterator<Item = usize> {
    (0..GAMEPLAY_PROJECTILE_RECYCLES_PER_FRAME).map(move |index| {
        GAMEPLAY_PROJECTILE_START + cohort + index * GAMEPLAY_PROJECTILE_LIFETIME_FRAMES
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GameplayDigest {
    pub actual_entity_count: usize,
    pub unique_mapped_entity_count: usize,
    pub moving_count: usize,
    pub health_count: usize,
    pub lifetime_count: usize,
    pub stunned_count: usize,
    pub component_mask_checksum: u64,
    pub position_checksum: u64,
    pub velocity_checksum: u64,
    pub health_checksum: u64,
    pub damage_checksum: u64,
    pub regen_checksum: u64,
    pub cooldown_checksum: u64,
    pub lifetime_checksum: u64,
    pub target_slot_checksum: u64,
    pub owner_slot_checksum: u64,
    pub cooldown_trace_checksum: u64,
    pub generation_checksum: u64,
    pub ai_lookup_checksum: u64,
}

/// Canonical state after running [`GameplayTrace::standard`] for all 256
/// frames. Adapter validation compares against this value, not against another
/// ECS implementation.
pub const GAMEPLAY_CANONICAL_DIGEST: GameplayDigest = GameplayDigest {
    actual_entity_count: 65_536,
    unique_mapped_entity_count: 65_536,
    moving_count: 53_248,
    health_count: 24_576,
    lifetime_count: 12_288,
    stunned_count: 1_024,
    component_mask_checksum: 13_341_931_166_528_947_233,
    position_checksum: 10_560_141_441_565_265_326,
    velocity_checksum: 1_395_278_975_465_285_033,
    health_checksum: 9_148_023_443_901_091_336,
    damage_checksum: 16_656_466_140_374_394_388,
    regen_checksum: 14_036_302_533_298_054_945,
    cooldown_checksum: 14_603_649_943_731_757_795,
    lifetime_checksum: 8_703_545_528_283_948_197,
    target_slot_checksum: 9_422_107_194_792_689_560,
    owner_slot_checksum: 722_449_847_523_312_062,
    cooldown_trace_checksum: 8_983_881_960_368_478_124,
    generation_checksum: 9_000_274_208_603_740_793,
    ai_lookup_checksum: 6_326_298_520_588_265_962,
};

#[derive(Clone)]
struct ReferenceEntity {
    position: PositionComponent,
    velocity: Option<VelocityComponent>,
    health: Option<Health>,
    damage: Option<Damage>,
    regen: Option<Regen>,
    enemy: bool,
    ally: bool,
    lifetime: Option<Lifetime>,
    target: Option<TargetSlot>,
    cooldown: Option<Cooldown>,
    owner: Option<OwnerSlot>,
    stunned: bool,
    tag_a: bool,
    tag_b: bool,
    generation: u32,
}

/// ECS-independent oracle for adapter validation. It intentionally stores
/// entities by logical slot rather than imitating an archetype implementation.
pub struct GameplayReference {
    entities: Vec<ReferenceEntity>,
    target_slots: Vec<usize>,
    ai_lookup_checksum: u64,
    cooldown_trace_checksum: u64,
}

impl GameplayReference {
    pub fn new(trace: &GameplayTrace) -> Self {
        let mut initially_stunned = vec![false; GAMEPLAY_ENTITY_COUNT];
        for &slot in trace.initially_stunned() {
            initially_stunned[slot] = true;
        }
        let entities = (0..GAMEPLAY_ENTITY_COUNT)
            .map(|slot| {
                let values = gameplay_entity_values(slot, 0, initially_stunned[slot]);
                ReferenceEntity {
                    position: values.position,
                    velocity: values.velocity,
                    health: values.health,
                    damage: values.damage,
                    regen: values.regen,
                    enemy: values.enemy.is_some(),
                    ally: values.ally.is_some(),
                    lifetime: values.lifetime,
                    target: values.target,
                    cooldown: values.cooldown,
                    owner: values.owner,
                    stunned: values.stunned.is_some(),
                    tag_a: matches!(gameplay_slot_spec(slot).variant, 1 | 3),
                    tag_b: matches!(gameplay_slot_spec(slot).variant, 2 | 3),
                    generation: 0,
                }
            })
            .collect();
        Self {
            entities,
            target_slots: Vec::with_capacity(GAMEPLAY_AI_LOOKUPS_PER_FRAME),
            ai_lookup_checksum: 0,
            cooldown_trace_checksum: 0,
        }
    }

    pub fn run_trace(&mut self, trace: &GameplayTrace) {
        for frame in trace.frames() {
            self.run_frame(frame);
        }
    }

    pub fn run_frame(&mut self, frame: &GameplayFrame) {
        for phase in super::gameplay_phase::GameplayPhase::ALL {
            self.run_phase(phase, frame);
        }
    }

    pub fn run_phase(
        &mut self,
        phase: super::gameplay_phase::GameplayPhase,
        frame: &GameplayFrame,
    ) {
        use super::gameplay_phase::GameplayPhase;

        match phase {
            GameplayPhase::Iteration => self.run_iteration_phase(),
            GameplayPhase::AiSourceLookup => self.run_ai_source_phase(frame),
            GameplayPhase::TargetPositionLookup => self.run_target_position_phase(frame),
            GameplayPhase::StatusTransition => self.run_status_transition_phase(frame),
            GameplayPhase::ProjectileRecycle => self.run_projectile_recycle_phase(frame),
        }
    }

    fn run_iteration_phase(&mut self) {
        for entity in &mut self.entities {
            if let Some(velocity) = entity.velocity {
                entity.position.0 += velocity.0;
            }
        }

        for entity in &mut self.entities[GAMEPLAY_COMBAT_START..GAMEPLAY_AI_START] {
            if let (Some(health), Some(damage)) = (&mut entity.health, entity.damage) {
                health.0 -= damage.0;
            } else if let (Some(health), Some(regen)) = (&mut entity.health, entity.regen) {
                health.0 += regen.0;
            }
        }

        for entity in &mut self.entities {
            let is_effect = entity.velocity.is_none();
            if let Some(lifetime) = &mut entity.lifetime {
                lifetime.0 = lifetime.0.saturating_sub(1);
                if lifetime.0 == 0 && is_effect {
                    lifetime.0 = 256;
                }
            }
        }
    }

    fn run_ai_source_phase(&mut self, frame: &GameplayFrame) {
        self.target_slots.clear();
        for &slot in frame.ai_slots.iter() {
            let entity = &mut self.entities[slot];
            let target = entity.target.expect("AI entity must have TargetSlot").0 as usize;
            let cooldown = entity
                .cooldown
                .as_mut()
                .expect("AI entity must have Cooldown");
            cooldown.0 = cooldown.0.saturating_sub(1);
            self.cooldown_trace_checksum = gameplay_ai_trace_checksum(
                self.cooldown_trace_checksum,
                frame.index,
                slot,
                target,
                cooldown.0,
            );
            self.target_slots.push(target);
        }
    }

    fn run_target_position_phase(&mut self, frame: &GameplayFrame) {
        assert_eq!(self.target_slots.len(), frame.ai_slots.len());
        for (&slot, &target) in frame.ai_slots.iter().zip(&self.target_slots) {
            let position = self.entities[target].position;
            self.ai_lookup_checksum =
                gameplay_ai_lookup_checksum(self.ai_lookup_checksum, slot, target, &position);
        }
    }

    fn run_status_transition_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.remove_stunned.iter() {
            assert!(
                self.entities[slot].stunned,
                "removing absent Stunned at slot {slot}"
            );
            self.entities[slot].stunned = false;
        }
        for &slot in frame.add_stunned.iter() {
            assert!(
                !self.entities[slot].stunned,
                "adding duplicate Stunned at slot {slot}"
            );
            self.entities[slot].stunned = true;
        }
    }

    fn run_projectile_recycle_phase(&mut self, frame: &GameplayFrame) {
        for &slot in frame.recycle_projectiles.iter() {
            let generation = self.entities[slot].generation.wrapping_add(1);
            let values = gameplay_entity_values(slot, generation, false);
            self.entities[slot] = ReferenceEntity {
                position: values.position,
                velocity: values.velocity,
                health: values.health,
                damage: values.damage,
                regen: values.regen,
                enemy: values.enemy.is_some(),
                ally: values.ally.is_some(),
                lifetime: values.lifetime,
                target: values.target,
                cooldown: values.cooldown,
                owner: values.owner,
                stunned: false,
                tag_a: matches!(gameplay_slot_spec(slot).variant, 1 | 3),
                tag_b: matches!(gameplay_slot_spec(slot).variant, 2 | 3),
                generation,
            };
        }
    }

    pub fn digest(&self) -> GameplayDigest {
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

        for (slot, entity) in self.entities.iter().enumerate() {
            moving_count += usize::from(entity.velocity.is_some());
            health_count += usize::from(entity.health.is_some());
            lifetime_count += usize::from(entity.lifetime.is_some());
            stunned_count += usize::from(entity.stunned);
            let mask = reference_component_mask(entity);
            value_checksums.observe(
                slot,
                entity.velocity.as_ref(),
                entity.damage.as_ref(),
                entity.regen.as_ref(),
                entity.cooldown.as_ref(),
            );
            component_mask_checksum =
                gameplay_mix_checksum(component_mask_checksum, slot as u64, mask as u64);
            position_checksum = gameplay_mix_checksum(
                position_checksum,
                slot as u64,
                (entity.position.0.x.to_bits() as u64)
                    ^ ((entity.position.0.y.to_bits() as u64) << 1)
                    ^ ((entity.position.0.z.to_bits() as u64) << 2),
            );
            if let Some(health) = entity.health {
                health_checksum =
                    gameplay_mix_checksum(health_checksum, slot as u64, health.0.to_bits() as u64);
            }
            if let Some(lifetime) = entity.lifetime {
                lifetime_checksum =
                    gameplay_mix_checksum(lifetime_checksum, slot as u64, lifetime.0 as u64);
            }
            if let Some(target) = entity.target {
                target_slot_checksum =
                    gameplay_mix_checksum(target_slot_checksum, slot as u64, target.0 as u64);
            }
            if let Some(owner) = entity.owner {
                owner_slot_checksum =
                    gameplay_mix_checksum(owner_slot_checksum, slot as u64, owner.0 as u64);
            }
            generation_checksum =
                gameplay_mix_checksum(generation_checksum, slot as u64, entity.generation as u64);
        }

        GameplayDigest {
            actual_entity_count: self.entities.len(),
            unique_mapped_entity_count: self.entities.len(),
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

#[derive(Default)]
pub struct GameplayValueChecksums {
    pub velocity: u64,
    pub damage: u64,
    pub regen: u64,
    pub cooldown: u64,
}

impl GameplayValueChecksums {
    pub fn observe(
        &mut self,
        slot: usize,
        velocity: Option<&VelocityComponent>,
        damage: Option<&Damage>,
        regen: Option<&Regen>,
        cooldown: Option<&Cooldown>,
    ) {
        if let Some(velocity) = velocity {
            self.observe_velocity(slot, velocity);
        }
        if let Some(damage) = damage {
            self.observe_damage(slot, damage);
        }
        if let Some(regen) = regen {
            self.observe_regen(slot, regen);
        }
        if let Some(cooldown) = cooldown {
            self.observe_cooldown(slot, cooldown);
        }
    }

    pub fn observe_velocity(&mut self, slot: usize, velocity: &VelocityComponent) {
        self.velocity = gameplay_mix_checksum(
            self.velocity,
            slot as u64,
            (velocity.0.x.to_bits() as u64)
                ^ ((velocity.0.y.to_bits() as u64) << 1)
                ^ ((velocity.0.z.to_bits() as u64) << 2),
        );
    }

    pub fn observe_damage(&mut self, slot: usize, damage: &Damage) {
        self.damage = gameplay_mix_checksum(self.damage, slot as u64, damage.0.to_bits() as u64);
    }

    pub fn observe_regen(&mut self, slot: usize, regen: &Regen) {
        self.regen = gameplay_mix_checksum(self.regen, slot as u64, regen.0.to_bits() as u64);
    }

    pub fn observe_cooldown(&mut self, slot: usize, cooldown: &Cooldown) {
        self.cooldown = gameplay_mix_checksum(self.cooldown, slot as u64, cooldown.0 as u64);
    }
}

fn reference_component_mask(entity: &ReferenceEntity) -> u16 {
    GAMEPLAY_MASK_POSITION
        | if entity.velocity.is_some() {
            GAMEPLAY_MASK_VELOCITY
        } else {
            0
        }
        | if entity.health.is_some() {
            GAMEPLAY_MASK_HEALTH
        } else {
            0
        }
        | if entity.damage.is_some() {
            GAMEPLAY_MASK_DAMAGE
        } else {
            0
        }
        | if entity.regen.is_some() {
            GAMEPLAY_MASK_REGEN
        } else {
            0
        }
        | if entity.enemy { GAMEPLAY_MASK_ENEMY } else { 0 }
        | if entity.ally { GAMEPLAY_MASK_ALLY } else { 0 }
        | if entity.lifetime.is_some() {
            GAMEPLAY_MASK_LIFETIME
        } else {
            0
        }
        | if entity.target.is_some() {
            GAMEPLAY_MASK_TARGET
        } else {
            0
        }
        | if entity.cooldown.is_some() {
            GAMEPLAY_MASK_COOLDOWN
        } else {
            0
        }
        | if entity.owner.is_some() {
            GAMEPLAY_MASK_OWNER
        } else {
            0
        }
        | if entity.stunned {
            GAMEPLAY_MASK_STUNNED
        } else {
            0
        }
        | if entity.tag_a { GAMEPLAY_MASK_TAG_A } else { 0 }
        | if entity.tag_b { GAMEPLAY_MASK_TAG_B } else { 0 }
}

#[inline]
pub fn gameplay_position_bits(position: &PositionComponent) -> u64 {
    (position.0.x.to_bits() as u64)
        ^ (position.0.y.to_bits() as u64).rotate_left(21)
        ^ (position.0.z.to_bits() as u64).rotate_left(42)
}

#[inline]
pub fn gameplay_ai_trace_checksum(
    checksum: u64,
    frame: usize,
    slot: usize,
    target: usize,
    cooldown_after: u32,
) -> u64 {
    let checksum = gameplay_mix_checksum(checksum, frame as u64, slot as u64);
    let checksum = gameplay_mix_checksum(checksum, slot as u64, target as u64);
    gameplay_mix_checksum(checksum, target as u64, cooldown_after as u64)
}

#[inline]
pub fn gameplay_ai_lookup_checksum(
    checksum: u64,
    slot: usize,
    target: usize,
    position: &PositionComponent,
) -> u64 {
    let checksum = gameplay_mix_checksum(checksum, slot as u64, target as u64);
    gameplay_mix_checksum(checksum, target as u64, gameplay_position_bits(position))
}

#[inline]
pub fn gameplay_mix_checksum(checksum: u64, slot: u64, value: u64) -> u64 {
    checksum
        .rotate_left(7)
        .wrapping_add(slot.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        ^ value.wrapping_mul(0xbf58_476d_1ce4_e5b9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameplay_layout_matches_published_contract() {
        assert_eq!(
            GAMEPLAY_EFFECT_START + GAMEPLAY_EFFECT_COUNT,
            GAMEPLAY_ENTITY_COUNT
        );
        assert_eq!(GAMEPLAY_MOVING_COUNT, 53_248);
        assert_eq!(GAMEPLAY_HEALTH_COUNT, 24_576);
        assert_eq!(GAMEPLAY_LIFETIME_COUNT, 12_288);

        let trace = GameplayTrace::standard();
        assert_eq!(trace.frames().len(), GAMEPLAY_FRAME_COUNT);
        assert_eq!(trace.initially_stunned().len(), GAMEPLAY_STUNNED_COUNT);
        for frame in trace.frames() {
            assert_eq!(frame.ai_slots.len(), GAMEPLAY_AI_LOOKUPS_PER_FRAME);
            assert_eq!(
                frame.remove_stunned.len(),
                GAMEPLAY_STATUS_CHANGES_PER_FRAME
            );
            assert_eq!(frame.add_stunned.len(), GAMEPLAY_STATUS_CHANGES_PER_FRAME);
            assert_eq!(
                frame.recycle_projectiles.len(),
                GAMEPLAY_PROJECTILE_RECYCLES_PER_FRAME
            );
        }
    }

    #[test]
    fn gameplay_reference_preserves_counts_for_256_frames() {
        let trace = GameplayTrace::standard();
        let mut reference = GameplayReference::new(&trace);
        reference.run_trace(&trace);
        let digest = reference.digest();
        assert_eq!(digest, GAMEPLAY_CANONICAL_DIGEST);
        assert_eq!(digest.actual_entity_count, GAMEPLAY_ENTITY_COUNT);
        assert_eq!(digest.unique_mapped_entity_count, GAMEPLAY_ENTITY_COUNT);
        assert_eq!(digest.moving_count, GAMEPLAY_MOVING_COUNT);
        assert_eq!(digest.health_count, GAMEPLAY_HEALTH_COUNT);
        assert_eq!(digest.lifetime_count, GAMEPLAY_LIFETIME_COUNT);
        assert_eq!(digest.stunned_count, GAMEPLAY_STUNNED_COUNT);
    }

    #[test]
    fn structural_cohorts_have_real_lifetimes() {
        let trace = GameplayTrace::standard();
        for frame in trace.frames() {
            assert!(frame
                .add_stunned
                .iter()
                .all(|slot| !frame.remove_stunned.contains(slot)));
        }
        assert_eq!(
            trace.frames()[0].add_stunned.as_ref(),
            trace.frames()[GAMEPLAY_STATUS_DURATION_FRAMES]
                .remove_stunned
                .as_ref()
        );
        assert_eq!(
            trace.frames()[0].recycle_projectiles.as_ref(),
            trace.frames()[GAMEPLAY_PROJECTILE_LIFETIME_FRAMES]
                .recycle_projectiles
                .as_ref()
        );
    }

    #[test]
    fn gameplay_digest_rejects_omitted_or_misdirected_work() {
        let trace = GameplayTrace::standard();
        let mut canonical = GameplayReference::new(&trace);
        canonical.run_frame(&trace.frames()[0]);
        let expected = canonical.digest();

        let mut no_tags = GameplayReference::new(&trace);
        no_tags.entities[1].tag_a = false;
        no_tags.run_frame(&trace.frames()[0]);
        assert_ne!(no_tags.digest(), expected);

        let mut no_owner = GameplayReference::new(&trace);
        no_owner.entities[GAMEPLAY_PROJECTILE_START + 1].owner = None;
        no_owner.run_frame(&trace.frames()[0]);
        assert_ne!(no_owner.digest(), expected);

        let mut wrong_target = GameplayReference::new(&trace);
        let ai = trace.frames()[0].ai_slots[0];
        wrong_target.entities[ai].target = Some(TargetSlot(
            (wrong_target.entities[ai].target.unwrap().0 + 256) % GAMEPLAY_PROJECTILE_START as u32,
        ));
        wrong_target.run_frame(&trace.frames()[0]);
        assert_ne!(wrong_target.digest(), expected);

        let mut wrong_stunned = GameplayReference::new(&trace);
        let left = trace.frames()[0].remove_stunned[0];
        let right = trace.frames()[0].add_stunned[0];
        wrong_stunned.entities[left].stunned = false;
        wrong_stunned.entities[right].stunned = true;
        assert_ne!(
            wrong_stunned.digest(),
            GameplayReference::new(&trace).digest()
        );

        let mut skipped_cooldown = expected;
        skipped_cooldown.cooldown_trace_checksum = 0;
        assert_ne!(skipped_cooldown, expected);

        let mut leaked_projectile = expected;
        leaked_projectile.actual_entity_count += 1;
        assert_ne!(leaked_projectile, expected);

        let mut wrong_damage = GameplayReference::new(&trace);
        wrong_damage.entities[GAMEPLAY_COMBAT_START].damage = Some(Damage(123.0));
        wrong_damage.run_frame(&trace.frames()[0]);
        assert_ne!(wrong_damage.digest(), expected);

        let mut duplicated_mapping = expected;
        duplicated_mapping.unique_mapped_entity_count -= 1;
        assert_ne!(duplicated_mapping, expected);
    }
}

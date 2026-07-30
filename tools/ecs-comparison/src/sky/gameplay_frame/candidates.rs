use super::*;
use sky_ecs::{PreparedEntityAccessor, PreparedEntityView};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IterationCandidate {
    Closure,
    Function,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiCandidate {
    WorldGetPair,
    SplitAccessors,
    PreparedEntityView,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionCandidate {
    WorldGet,
    EntityAccessor,
    PreparedEntityAccessor,
    PreparedEntityView,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameCandidateSelection {
    pub iteration: IterationCandidate,
    pub ai: AiCandidate,
    pub position: PositionCandidate,
}

impl FrameCandidateSelection {
    pub const PRODUCTION: Self = Self {
        iteration: IterationCandidate::Closure,
        ai: AiCandidate::PreparedEntityView,
        position: PositionCandidate::PreparedEntityAccessor,
    };
}

struct CandidateWorld {
    inner: SkyGameplayWorld,
    position_accessor: PreparedEntityAccessor<PositionComponent>,
    position_view: PreparedEntityView<&'static PositionComponent>,
}

impl CandidateWorld {
    fn new(trace: &GameplayTrace) -> Self {
        Self {
            inner: SkyGameplayWorld::new(trace),
            position_accessor: PreparedEntityAccessor::new(),
            position_view: PreparedEntityView::new(),
        }
    }

    fn iteration(&mut self, candidate: IterationCandidate) {
        match candidate {
            IterationCandidate::Closure => self.inner.run_iteration_phase(),
            IterationCandidate::Function => {
                self.inner
                    .movement
                    .for_each_chunk(&mut self.inner.world, move_chunk);
                self.inner
                    .enemies
                    .for_each_chunk(&mut self.inner.world, damage_chunk);
                self.inner
                    .allies
                    .for_each_chunk(&mut self.inner.world, regen_chunk);
                self.inner
                    .lifetimes
                    .for_each_chunk(&mut self.inner.world, lifetime_chunk);
            }
        }
    }

    fn ai(&mut self, frame: &GameplayFrame, candidate: AiCandidate) {
        self.inner.target_entities.clear();
        self.inner.target_slots.clear();
        match candidate {
            AiCandidate::WorldGetPair => {
                for &slot in frame.ai_slots.iter() {
                    let entity = self.inner.entities[slot];
                    let target_slot = self
                        .inner
                        .world
                        .get::<TargetSlot>(entity)
                        .expect("AI TargetSlot")
                        .0 as usize;
                    self.inner
                        .target_entities
                        .push(self.inner.entities[target_slot]);
                    self.inner.target_slots.push(target_slot);
                    let cooldown = self
                        .inner
                        .world
                        .get_mut::<Cooldown>(entity)
                        .expect("AI Cooldown");
                    cooldown.0 = cooldown.0.saturating_sub(1);
                    self.inner.cooldown_trace_checksum = gameplay_ai_trace_checksum(
                        self.inner.cooldown_trace_checksum,
                        frame.index,
                        slot,
                        target_slot,
                        cooldown.0,
                    );
                }
            }
            AiCandidate::SplitAccessors => {
                {
                    let targets = self.inner.world.accessor::<TargetSlot>();
                    for &slot in frame.ai_slots.iter() {
                        let target_slot =
                            targets.get(self.inner.entities[slot]).unwrap().0 as usize;
                        self.inner
                            .target_entities
                            .push(self.inner.entities[target_slot]);
                        self.inner.target_slots.push(target_slot);
                    }
                }
                {
                    let mut cooldowns = self.inner.world.accessor_mut::<Cooldown>();
                    for (&slot, &target_slot) in frame.ai_slots.iter().zip(&self.inner.target_slots)
                    {
                        let entity = self.inner.entities[slot];
                        let cooldown = cooldowns.get_mut(entity).unwrap();
                        cooldown.0 = cooldown.0.saturating_sub(1);
                        self.inner.cooldown_trace_checksum = gameplay_ai_trace_checksum(
                            self.inner.cooldown_trace_checksum,
                            frame.index,
                            slot,
                            target_slot,
                            cooldown.0,
                        );
                    }
                }
            }
            AiCandidate::PreparedEntityView => self.inner.run_ai_source_phase(frame),
        }
    }

    fn position(&mut self, frame: &GameplayFrame, candidate: PositionCandidate) {
        match candidate {
            PositionCandidate::WorldGet => {
                for ((&slot, &entity), &target_slot) in frame
                    .ai_slots
                    .iter()
                    .zip(&self.inner.target_entities)
                    .zip(&self.inner.target_slots)
                {
                    let position = self.inner.world.get::<PositionComponent>(entity).unwrap();
                    self.inner.ai_lookup_checksum = gameplay_ai_lookup_checksum(
                        self.inner.ai_lookup_checksum,
                        slot,
                        target_slot,
                        position,
                    );
                }
            }
            PositionCandidate::EntityAccessor => self.inner.run_target_position_phase(frame),
            PositionCandidate::PreparedEntityAccessor => {
                let positions = self.position_accessor.bind(&self.inner.world);
                for ((&slot, &entity), &target_slot) in frame
                    .ai_slots
                    .iter()
                    .zip(&self.inner.target_entities)
                    .zip(&self.inner.target_slots)
                {
                    let position = positions.get(entity).unwrap();
                    self.inner.ai_lookup_checksum = gameplay_ai_lookup_checksum(
                        self.inner.ai_lookup_checksum,
                        slot,
                        target_slot,
                        position,
                    );
                }
            }
            PositionCandidate::PreparedEntityView => {
                let positions = self.position_view.bind(&self.inner.world);
                for ((&slot, &entity), &target_slot) in frame
                    .ai_slots
                    .iter()
                    .zip(&self.inner.target_entities)
                    .zip(&self.inner.target_slots)
                {
                    let position = positions.get(entity).unwrap();
                    self.inner.ai_lookup_checksum = gameplay_ai_lookup_checksum(
                        self.inner.ai_lookup_checksum,
                        slot,
                        target_slot,
                        position,
                    );
                }
            }
        }
    }

    fn phase(
        &mut self,
        phase: GameplayPhase,
        frame: &GameplayFrame,
        selection: FrameCandidateSelection,
    ) {
        match phase {
            GameplayPhase::Iteration => self.iteration(selection.iteration),
            GameplayPhase::AiSourceLookup => self.ai(frame, selection.ai),
            GameplayPhase::TargetPositionLookup => self.position(frame, selection.position),
            GameplayPhase::StatusTransition => self.inner.run_status_transition_phase(frame),
            GameplayPhase::ProjectileRecycle => self.inner.run_projectile_recycle_phase(frame),
        }
    }
}

fn measure(selection: FrameCandidateSelection, measured: Option<GameplayPhase>) -> Duration {
    let trace = GameplayTrace::standard();
    let mut world = CandidateWorld::new(&trace);
    let mut elapsed = Duration::ZERO;
    for frame in trace.frames() {
        if measured.is_none() {
            let start = Instant::now();
            for phase in GameplayPhase::ALL {
                world.phase(phase, frame, selection);
            }
            elapsed += start.elapsed();
        } else {
            for phase in GameplayPhase::ALL {
                if measured == Some(phase) {
                    let start = Instant::now();
                    world.phase(phase, frame, selection);
                    elapsed += start.elapsed();
                } else {
                    world.phase(phase, frame, selection);
                }
            }
        }
    }
    assert_eq!(world.inner.digest(), GAMEPLAY_CANONICAL_DIGEST);
    black_box(&world);
    elapsed
}

pub fn measure_iteration_candidate(candidate: IterationCandidate) -> Duration {
    measure(
        FrameCandidateSelection {
            iteration: candidate,
            ..FrameCandidateSelection::PRODUCTION
        },
        Some(GameplayPhase::Iteration),
    )
}

pub fn measure_ai_candidate(candidate: AiCandidate) -> Duration {
    measure(
        FrameCandidateSelection {
            ai: candidate,
            ..FrameCandidateSelection::PRODUCTION
        },
        Some(GameplayPhase::AiSourceLookup),
    )
}

pub fn measure_position_candidate(candidate: PositionCandidate) -> Duration {
    measure(
        FrameCandidateSelection {
            position: candidate,
            ..FrameCandidateSelection::PRODUCTION
        },
        Some(GameplayPhase::TargetPositionLookup),
    )
}

pub fn measure_frame_candidate(selection: FrameCandidateSelection) -> Duration {
    measure(selection, None)
}

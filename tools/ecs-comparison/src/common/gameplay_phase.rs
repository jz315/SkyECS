use super::{
    GameplayDigest, GameplayFrame, GameplayReference, GameplayTrace, GAMEPLAY_CANONICAL_DIGEST,
    GAMEPLAY_CONTRACT_CHECKPOINTS,
};
use criterion::{measurement::WallTime, BenchmarkGroup};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameplayPhase {
    Iteration,
    AiSourceLookup,
    TargetPositionLookup,
    StatusTransition,
    ProjectileRecycle,
}

impl GameplayPhase {
    pub const ALL: [Self; 5] = [
        Self::Iteration,
        Self::AiSourceLookup,
        Self::TargetPositionLookup,
        Self::StatusTransition,
        Self::ProjectileRecycle,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Iteration => "iteration",
            Self::AiSourceLookup => "ai_source_lookup",
            Self::TargetPositionLookup => "target_position_lookup",
            Self::StatusTransition => "status_transition",
            Self::ProjectileRecycle => "projectile_recycle",
        }
    }
}

/// Shared phase contract used by full-frame and diagnostic gameplay paths.
pub trait GameplayPhaseAdapter {
    fn run_phase(&mut self, phase: GameplayPhase, frame: &GameplayFrame);

    fn digest(&self) -> GameplayDigest;

    #[inline]
    fn run_frame(&mut self, frame: &GameplayFrame) {
        for phase in GameplayPhase::ALL {
            self.run_phase(phase, frame);
        }
    }
}

pub fn validate_gameplay_runner<Adapter, Factory, Run, Digest>(
    factory: Factory,
    run_frame: Run,
    digest: Digest,
) where
    Factory: FnOnce(&GameplayTrace) -> Adapter,
    Run: Fn(&mut Adapter, &GameplayFrame),
    Digest: Fn(&Adapter) -> GameplayDigest,
{
    let trace = GameplayTrace::standard();
    let mut expected = GameplayReference::new(&trace);
    let mut actual = factory(&trace);
    for frame in trace.frames() {
        expected.run_frame(frame);
        run_frame(&mut actual, frame);
        if GAMEPLAY_CONTRACT_CHECKPOINTS.contains(&frame.index) {
            assert_eq!(
                digest(&actual),
                expected.digest(),
                "gameplay contract diverged after frame {}",
                frame.index
            );
        }
    }
    assert_eq!(expected.digest(), GAMEPLAY_CANONICAL_DIGEST);
    assert_eq!(digest(&actual), GAMEPLAY_CANONICAL_DIGEST);
}

pub fn validate_gameplay_adapter<Adapter, Factory>(factory: Factory)
where
    Adapter: GameplayPhaseAdapter,
    Factory: FnOnce(&GameplayTrace) -> Adapter,
{
    const PHASE_CHECKPOINTS: &[usize] = &[0, 7, 63, 255];

    let trace = GameplayTrace::standard();
    let mut expected = GameplayReference::new(&trace);
    let mut actual = factory(&trace);
    for frame in trace.frames() {
        for phase in GameplayPhase::ALL {
            expected.run_phase(phase, frame);
            actual.run_phase(phase, frame);
            if PHASE_CHECKPOINTS.contains(&frame.index) {
                assert_eq!(
                    actual.digest(),
                    expected.digest(),
                    "gameplay contract diverged after frame {}, phase {:?}",
                    frame.index,
                    phase
                );
            }
        }
    }
    assert_eq!(expected.digest(), GAMEPLAY_CANONICAL_DIGEST);
    assert_eq!(actual.digest(), GAMEPLAY_CANONICAL_DIGEST);
}

/// Benchmarks one phase while still executing the complete evolving frame.
///
/// Context construction, untimed phases, digest validation, and trace resets
/// are excluded from the returned duration. The measurements are diagnostic
/// timing windows and are not claimed to add exactly to the full-frame row.
pub fn bench_gameplay_phases<Adapter, Factory>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    engine: &'static str,
    factory: Factory,
) where
    Adapter: GameplayPhaseAdapter,
    Factory: Fn(&GameplayTrace) -> Adapter + Copy + 'static,
{
    for measured_phase in GameplayPhase::ALL {
        group.bench_function(
            format!("{}/{}", measured_phase.name(), engine),
            move |bencher| {
                let trace = GameplayTrace::standard();
                bencher.iter_custom(|iterations| {
                    let mut gameplay = factory(&trace);
                    let mut frame_index = 0usize;
                    let mut measured = Duration::ZERO;

                    for _ in 0..iterations {
                        let frame = &trace.frames()[frame_index];
                        for phase in GameplayPhase::ALL {
                            if phase == measured_phase {
                                let start = Instant::now();
                                gameplay.run_phase(phase, frame);
                                measured += start.elapsed();
                            } else {
                                gameplay.run_phase(phase, frame);
                            }
                        }

                        frame_index += 1;
                        if frame_index == trace.frames().len() {
                            assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
                            gameplay = factory(&trace);
                            frame_index = 0;
                        }
                    }

                    black_box(&gameplay);
                    measured
                });
            },
        );
    }
}

/// Benchmarks complete frames over the same resettable canonical trace used by
/// the phase timing windows. Context reset and digest validation stay outside
/// the duration returned to Criterion.
pub fn bench_full_gameplay_frames<Adapter, Factory, Run>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    engine: &'static str,
    factory: Factory,
    run_frame: Run,
) where
    Adapter: GameplayPhaseAdapter,
    Factory: Fn(&GameplayTrace) -> Adapter + Copy + 'static,
    Run: Fn(&mut Adapter, &GameplayFrame) + Copy + 'static,
{
    group.bench_function(format!("frame/{engine}"), move |bencher| {
        let trace = GameplayTrace::standard();
        bencher.iter_custom(|iterations| {
            let mut gameplay = factory(&trace);
            let mut frame_index = 0usize;
            let mut measured = Duration::ZERO;

            for _ in 0..iterations {
                let frame = &trace.frames()[frame_index];
                let start = Instant::now();
                run_frame(&mut gameplay, frame);
                measured += start.elapsed();

                frame_index += 1;
                if frame_index == trace.frames().len() {
                    assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
                    gameplay = factory(&trace);
                    frame_index = 0;
                }
            }

            black_box(&gameplay);
            measured
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ShiftedAiWork {
        reference: GameplayReference,
        pending_frame: Option<GameplayFrame>,
    }

    impl GameplayPhaseAdapter for ShiftedAiWork {
        fn run_phase(&mut self, phase: GameplayPhase, frame: &GameplayFrame) {
            match phase {
                GameplayPhase::AiSourceLookup => self.pending_frame = Some(frame.clone()),
                GameplayPhase::TargetPositionLookup => {
                    let pending = self.pending_frame.take().unwrap();
                    self.reference
                        .run_phase(GameplayPhase::AiSourceLookup, &pending);
                    self.reference.run_phase(phase, frame);
                }
                _ => self.reference.run_phase(phase, frame),
            }
        }

        fn digest(&self) -> GameplayDigest {
            self.reference.digest()
        }
    }

    #[test]
    #[should_panic(expected = "phase AiSourceLookup")]
    fn phase_contract_rejects_work_shifted_to_a_later_phase() {
        validate_gameplay_adapter(|trace| ShiftedAiWork {
            reference: GameplayReference::new(trace),
            pending_frame: None,
        });
    }
}

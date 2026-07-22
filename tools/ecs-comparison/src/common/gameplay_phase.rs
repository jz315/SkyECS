use super::{GameplayDigest, GameplayFrame, GameplayTrace, GAMEPLAY_CANONICAL_DIGEST};
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

use super::gameplay_frame::{SkyAiApi, SkyGameplayWorld, SkyLookupApi, SELECTED_ITERATION_API};
use crate::common::{GameplayTrace, GAMEPLAY_CANONICAL_DIGEST, GAMEPLAY_FRAME_COUNT};
use std::hint::black_box;
use std::time::{Duration, Instant};

const CLEAR_WIN_RATIO: f64 = 0.98;
const CLEAR_LOSS_RATIO: f64 = 1.02;
const INCUMBENT_AI: SkyAiApi = SkyAiApi::WorldGetPair;
const INCUMBENT_POSITION: SkyLookupApi = SkyLookupApi::EntityAccessor;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PairwiseOutcome {
    FirstWins,
    Tie,
    SecondWins,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct PairwiseRound {
    pub ab_first_ns_per_frame: f64,
    pub ab_second_ns_per_frame: f64,
    pub ba_second_ns_per_frame: f64,
    pub ba_first_ns_per_frame: f64,
    pub order_neutral_ratio: f64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SkyApiCandidateResult {
    pub first: &'static str,
    pub second: &'static str,
    pub outcome: PairwiseOutcome,
    pub rounds: Vec<PairwiseRound>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SkyPhaseApiSelection {
    pub incumbent: &'static str,
    pub comparisons: Vec<SkyApiCandidateResult>,
    pub condorcet_winner: Option<&'static str>,
    pub provisional_winner: &'static str,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SkyFullFrameSelection {
    pub incumbent_ai: &'static str,
    pub incumbent_position: &'static str,
    pub proposed_ai: &'static str,
    pub proposed_position: &'static str,
    pub comparison: SkyApiCandidateResult,
    pub accepted: bool,
    pub selected_ai: &'static str,
    pub selected_position: &'static str,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SkyGameplayApiCertification {
    pub rounds: usize,
    pub traces_per_measurement: usize,
    pub frames_per_trace: usize,
    pub decision_band: [f64; 2],
    pub ai_source: SkyPhaseApiSelection,
    pub target_position: SkyPhaseApiSelection,
    pub full_frame: SkyFullFrameSelection,
}

trait Candidate: Copy + Eq {
    fn name(self) -> &'static str;
}

impl Candidate for SkyAiApi {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGetPair => "World::get + World::get_mut",
            Self::SplitAccessors => "split EntityAccessor passes",
            Self::PreparedEntityView => "PreparedEntityView<(&TargetSlot, &mut Cooldown)>",
        }
    }
}

impl Candidate for SkyLookupApi {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGet => "World::get<Position>",
            Self::EntityAccessor => "EntityAccessor<Position>::get",
            Self::PreparedEntityView => "PreparedEntityView<&Position>::get",
        }
    }
}

pub fn certify_gameplay_apis(
    rounds: usize,
    traces_per_measurement: usize,
) -> SkyGameplayApiCertification {
    assert_eq!(
        rounds, 4,
        "gameplay certification requires four AB/BA rounds"
    );
    assert!(traces_per_measurement > 0);
    let trace = GameplayTrace::standard();

    let (ai_source, proposed_ai) = certify_three_way(
        [
            SkyAiApi::WorldGetPair,
            SkyAiApi::SplitAccessors,
            SkyAiApi::PreparedEntityView,
        ],
        INCUMBENT_AI,
        rounds,
        |candidate| measure_ai_phase(&trace, traces_per_measurement, candidate),
    );
    let (target_position, proposed_position) = certify_three_way(
        [
            SkyLookupApi::WorldGet,
            SkyLookupApi::EntityAccessor,
            SkyLookupApi::PreparedEntityView,
        ],
        INCUMBENT_POSITION,
        rounds,
        |candidate| measure_position_phase(&trace, traces_per_measurement, candidate),
    );

    let comparison = compare_pair(
        "production incumbent",
        (INCUMBENT_AI, INCUMBENT_POSITION),
        "provisional phase winners",
        (proposed_ai, proposed_position),
        rounds,
        |(ai, position)| measure_full_frame(&trace, traces_per_measurement, ai, position),
    );
    let changed = proposed_ai != INCUMBENT_AI || proposed_position != INCUMBENT_POSITION;
    let accepted = changed && comparison.outcome == PairwiseOutcome::SecondWins;
    let (selected_ai, selected_position) = if accepted {
        (proposed_ai, proposed_position)
    } else {
        (INCUMBENT_AI, INCUMBENT_POSITION)
    };

    SkyGameplayApiCertification {
        rounds,
        traces_per_measurement,
        frames_per_trace: GAMEPLAY_FRAME_COUNT,
        decision_band: [CLEAR_WIN_RATIO, CLEAR_LOSS_RATIO],
        ai_source,
        target_position,
        full_frame: SkyFullFrameSelection {
            incumbent_ai: INCUMBENT_AI.name(),
            incumbent_position: INCUMBENT_POSITION.name(),
            proposed_ai: proposed_ai.name(),
            proposed_position: proposed_position.name(),
            comparison,
            accepted,
            selected_ai: selected_ai.name(),
            selected_position: selected_position.name(),
        },
    }
}

fn certify_three_way<C, Measure>(
    candidates: [C; 3],
    incumbent: C,
    rounds: usize,
    measure: Measure,
) -> (SkyPhaseApiSelection, C)
where
    C: Candidate,
    Measure: Fn(C) -> f64 + Copy,
{
    let comparisons = vec![
        compare_pair(
            candidates[0].name(),
            candidates[0],
            candidates[1].name(),
            candidates[1],
            rounds,
            measure,
        ),
        compare_pair(
            candidates[0].name(),
            candidates[0],
            candidates[2].name(),
            candidates[2],
            rounds,
            measure,
        ),
        compare_pair(
            candidates[1].name(),
            candidates[1],
            candidates[2].name(),
            candidates[2],
            rounds,
            measure,
        ),
    ];

    let condorcet = candidates
        .into_iter()
        .find(|candidate| wins_against_both(candidate.name(), &comparisons));
    let provisional = condorcet.unwrap_or(incumbent);
    (
        SkyPhaseApiSelection {
            incumbent: incumbent.name(),
            comparisons,
            condorcet_winner: condorcet.map(Candidate::name),
            provisional_winner: provisional.name(),
        },
        provisional,
    )
}

fn wins_against_both(candidate: &str, comparisons: &[SkyApiCandidateResult]) -> bool {
    comparisons
        .iter()
        .filter(|comparison| comparison.first == candidate || comparison.second == candidate)
        .all(|comparison| {
            (comparison.first == candidate && comparison.outcome == PairwiseOutcome::FirstWins)
                || (comparison.second == candidate
                    && comparison.outcome == PairwiseOutcome::SecondWins)
        })
}

fn compare_pair<C, Measure>(
    first_name: &'static str,
    first: C,
    second_name: &'static str,
    second: C,
    rounds: usize,
    measure: Measure,
) -> SkyApiCandidateResult
where
    C: Copy,
    Measure: Fn(C) -> f64,
{
    let mut results = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let ab_first = measure(first);
        let ab_second = measure(second);
        let ba_second = measure(second);
        let ba_first = measure(first);
        let ratio = ((ab_first / ab_second) * (ba_first / ba_second)).sqrt();
        results.push(PairwiseRound {
            ab_first_ns_per_frame: ab_first,
            ab_second_ns_per_frame: ab_second,
            ba_second_ns_per_frame: ba_second,
            ba_first_ns_per_frame: ba_first,
            order_neutral_ratio: ratio,
        });
    }

    let outcome = if results
        .iter()
        .all(|round| round.order_neutral_ratio < CLEAR_WIN_RATIO)
    {
        PairwiseOutcome::FirstWins
    } else if results
        .iter()
        .all(|round| round.order_neutral_ratio > CLEAR_LOSS_RATIO)
    {
        PairwiseOutcome::SecondWins
    } else {
        PairwiseOutcome::Tie
    };

    SkyApiCandidateResult {
        first: first_name,
        second: second_name,
        outcome,
        rounds: results,
    }
}

fn measure_ai_phase(trace: &GameplayTrace, traces: usize, candidate: SkyAiApi) -> f64 {
    measure_phase(trace, traces, |gameplay, frame| {
        gameplay.run_iteration_phase(SELECTED_ITERATION_API);
        let start = Instant::now();
        gameplay.run_ai_source_phase(frame, candidate);
        let elapsed = start.elapsed();
        gameplay.run_target_position_phase(frame, INCUMBENT_POSITION);
        gameplay.run_status_transition_phase(frame);
        gameplay.run_projectile_recycle_phase(frame);
        elapsed
    })
}

fn measure_position_phase(trace: &GameplayTrace, traces: usize, candidate: SkyLookupApi) -> f64 {
    measure_phase(trace, traces, |gameplay, frame| {
        gameplay.run_iteration_phase(SELECTED_ITERATION_API);
        gameplay.run_ai_source_phase(frame, INCUMBENT_AI);
        let start = Instant::now();
        gameplay.run_target_position_phase(frame, candidate);
        let elapsed = start.elapsed();
        gameplay.run_status_transition_phase(frame);
        gameplay.run_projectile_recycle_phase(frame);
        elapsed
    })
}

fn measure_phase<Run>(trace: &GameplayTrace, traces: usize, mut run: Run) -> f64
where
    Run: FnMut(&mut SkyGameplayWorld, &crate::common::GameplayFrame) -> Duration,
{
    let mut elapsed = Duration::ZERO;
    for _ in 0..traces {
        let mut gameplay = SkyGameplayWorld::new(trace);
        for frame in trace.frames() {
            elapsed += run(&mut gameplay, frame);
        }
        assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
        black_box(&gameplay);
    }
    elapsed.as_nanos() as f64 / (traces * GAMEPLAY_FRAME_COUNT) as f64
}

fn measure_full_frame(
    trace: &GameplayTrace,
    traces: usize,
    ai: SkyAiApi,
    position: SkyLookupApi,
) -> f64 {
    let mut elapsed = Duration::ZERO;
    for _ in 0..traces {
        let mut gameplay = SkyGameplayWorld::new(trace);
        for frame in trace.frames() {
            let start = Instant::now();
            gameplay.run_frame_with_apis(frame, SELECTED_ITERATION_API, ai, position);
            elapsed += start.elapsed();
        }
        assert_eq!(gameplay.digest(), GAMEPLAY_CANONICAL_DIGEST);
        black_box(&gameplay);
    }
    elapsed.as_nanos() as f64 / (traces * GAMEPLAY_FRAME_COUNT) as f64
}

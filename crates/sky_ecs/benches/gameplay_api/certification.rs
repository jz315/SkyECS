#[path = "certification/metadata.rs"]
mod metadata;
#[path = "certification/pairwise.rs"]
mod pairwise;

use super::GameplayFixture;
use metadata::Metadata;
use pairwise::{
    compare_pair, PairwiseResult, CLEAR_LOSS_RATIO, CLEAR_WIN_RATIO, EXTRA_ROUNDS, INITIAL_ROUNDS,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

const MEASURE_ITERATIONS: usize = 256;
const WARMUP_ITERATIONS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IterationApi {
    ChunkClosure,
    ChunkFunction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AiApi {
    WorldGetPair,
    SplitAccessors,
    PreparedEntityView,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PositionApi {
    WorldGet,
    EntityAccessor,
    PreparedEntityView,
}

trait Candidate: Copy + Eq {
    fn name(self) -> &'static str;
}

impl Candidate for IterationApi {
    fn name(self) -> &'static str {
        match self {
            Self::ChunkClosure => "PreparedQuery::for_each_chunk closure",
            Self::ChunkFunction => "PreparedQuery::for_each_chunk_fn function",
        }
    }
}

impl Candidate for AiApi {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGetPair => "World::get + World::get_mut",
            Self::SplitAccessors => "split EntityAccessor passes",
            Self::PreparedEntityView => "PreparedEntityView<(&TargetSlot, &mut Cooldown)>",
        }
    }
}

impl Candidate for PositionApi {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGet => "World::get<Position>",
            Self::EntityAccessor => "EntityAccessor<Position>::get",
            Self::PreparedEntityView => "PreparedEntityView<&Position>::get",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FrameSelection {
    iteration: IterationApi,
    ai: AiApi,
    position: PositionApi,
}

impl FrameSelection {
    pub(crate) const fn production() -> Self {
        Self {
            iteration: IterationApi::ChunkClosure,
            ai: AiApi::PreparedEntityView,
            position: PositionApi::EntityAccessor,
        }
    }

    pub(crate) const fn world_get_baseline() -> Self {
        Self {
            iteration: IterationApi::ChunkClosure,
            ai: AiApi::WorldGetPair,
            position: PositionApi::WorldGet,
        }
    }

    pub(crate) const fn split_accessor_path() -> Self {
        Self {
            iteration: IterationApi::ChunkClosure,
            ai: AiApi::SplitAccessors,
            position: PositionApi::EntityAccessor,
        }
    }

    pub(crate) const fn all_prepared_views() -> Self {
        Self {
            iteration: IterationApi::ChunkClosure,
            ai: AiApi::PreparedEntityView,
            position: PositionApi::PreparedEntityView,
        }
    }

    pub(crate) fn run(self, fixture: &mut GameplayFixture) {
        run_iteration(fixture, self.iteration);
        run_ai(fixture, self.ai);
        run_position(fixture, self.position);
    }

    fn label(self) -> String {
        format!(
            "{} | {} | {}",
            self.iteration.name(),
            self.ai.name(),
            self.position.name()
        )
    }
}

#[derive(Debug, Serialize)]
struct PhaseSelection {
    incumbent: String,
    comparisons: Vec<PairwiseResult>,
    condorcet_winner: Option<String>,
    selected: String,
    selection_reason: &'static str,
}

#[derive(Debug, Serialize)]
struct FullFrameGate {
    incumbent: String,
    proposed: String,
    comparison: Option<PairwiseResult>,
    accepted: bool,
    selected: String,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct Certification {
    metadata: Metadata,
    operation_contracts: BTreeMap<&'static str, &'static str>,
    initial_rounds: usize,
    extra_rounds_when_unclear: usize,
    iterations_per_measurement: usize,
    decision_band: [f64; 2],
    iteration: PhaseSelection,
    ai_source: PhaseSelection,
    target_position: PhaseSelection,
    full_frame: FullFrameGate,
}

pub(crate) fn run() {
    let (iteration, iteration_winner) = certify_candidates(
        [IterationApi::ChunkClosure, IterationApi::ChunkFunction],
        IterationApi::ChunkClosure,
        measure_iteration,
    );
    let (ai_source, ai_winner) = certify_candidates(
        [
            AiApi::WorldGetPair,
            AiApi::SplitAccessors,
            AiApi::PreparedEntityView,
        ],
        AiApi::PreparedEntityView,
        measure_ai,
    );
    let (target_position, position_winner) = certify_candidates(
        [
            PositionApi::WorldGet,
            PositionApi::EntityAccessor,
            PositionApi::PreparedEntityView,
        ],
        PositionApi::EntityAccessor,
        measure_position,
    );

    let incumbent = FrameSelection::production();
    let proposed = FrameSelection {
        iteration: iteration_winner,
        ai: ai_winner,
        position: position_winner,
    };
    let full_frame = if proposed == incumbent {
        FullFrameGate {
            incumbent: incumbent.label(),
            proposed: proposed.label(),
            comparison: None,
            accepted: true,
            selected: incumbent.label(),
            reason: "phase winners already equal the production selection",
        }
    } else {
        let comparison = compare_pair(
            incumbent.label(),
            incumbent,
            proposed.label(),
            proposed,
            measure_frame,
        );
        let accepted = comparison.is_clear() && comparison.winner_is(&proposed.label());
        FullFrameGate {
            incumbent: incumbent.label(),
            proposed: proposed.label(),
            selected: if accepted {
                proposed.label()
            } else {
                incumbent.label()
            },
            comparison: Some(comparison),
            accepted,
            reason: if accepted {
                "proposed combination clearly won the full-frame gate"
            } else {
                "full-frame gate was not a clear proposed-path win; retained incumbent"
            },
        }
    };

    let report = Certification {
        metadata: metadata::collect(),
        operation_contracts: BTreeMap::from([
            (
                "iteration",
                "update every Position+Velocity row through the same prepared chunk query",
            ),
            (
                "ai_source",
                "for 2,048 selected entities read TargetSlot, emit target EntityId, and decrement Cooldown",
            ),
            (
                "target_position",
                "consume the emitted target EntityIds, read Position, and update one checksum",
            ),
            (
                "full_frame",
                "run iteration, AI source, and target Position phases in that order",
            ),
        ]),
        initial_rounds: INITIAL_ROUNDS,
        extra_rounds_when_unclear: EXTRA_ROUNDS,
        iterations_per_measurement: MEASURE_ITERATIONS,
        decision_band: [CLEAR_WIN_RATIO, CLEAR_LOSS_RATIO],
        iteration,
        ai_source,
        target_position,
        full_frame,
    };

    let output = std::env::var_os("SKY_ECS_CERTIFICATION_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target").join("sky-ecs-gameplay-api-certification.json"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("certification output directory must be writable");
    }
    let json = serde_json::to_string_pretty(&report).expect("certification must serialize");
    fs::write(&output, format!("{json}\n")).expect("certification output must be writable");
    println!("wrote {}", output.display());
}

fn certify_candidates<C, const N: usize>(
    candidates: [C; N],
    incumbent: C,
    measure: fn(C) -> f64,
) -> (PhaseSelection, C)
where
    C: Candidate,
{
    let mut comparisons = Vec::new();
    for first_index in 0..N {
        for second_index in first_index + 1..N {
            comparisons.push(compare_pair(
                candidates[first_index].name().to_owned(),
                candidates[first_index],
                candidates[second_index].name().to_owned(),
                candidates[second_index],
                measure,
            ));
        }
    }

    let condorcet = candidates.iter().copied().find(|candidate| {
        let name = candidate.name();
        comparisons
            .iter()
            .filter(|comparison| comparison.first == name || comparison.second == name)
            .all(|comparison| comparison.winner_is(name))
    });
    let selected = condorcet.unwrap_or(incumbent);
    (
        PhaseSelection {
            incumbent: incumbent.name().to_owned(),
            comparisons,
            condorcet_winner: condorcet.map(|candidate| candidate.name().to_owned()),
            selected: selected.name().to_owned(),
            selection_reason: if condorcet.is_some() {
                "selected pairwise Condorcet winner"
            } else {
                "pairwise cycle; retained incumbent"
            },
        },
        selected,
    )
}

fn measure_iteration(candidate: IterationApi) -> f64 {
    measure_fixture(|fixture| run_iteration(fixture, candidate))
}

fn measure_ai(candidate: AiApi) -> f64 {
    measure_fixture(|fixture| run_ai(fixture, candidate))
}

fn measure_position(candidate: PositionApi) -> f64 {
    let mut fixture = GameplayFixture::new();
    fixture.ai_prepared_entity_view();
    measure_existing_fixture(fixture, |fixture| run_position(fixture, candidate))
}

fn measure_frame(selection: FrameSelection) -> f64 {
    measure_fixture(|fixture| selection.run(fixture))
}

fn measure_fixture(mut run: impl FnMut(&mut GameplayFixture)) -> f64 {
    measure_existing_fixture(GameplayFixture::new(), move |fixture| run(fixture))
}

fn measure_existing_fixture(
    mut fixture: GameplayFixture,
    mut run: impl FnMut(&mut GameplayFixture),
) -> f64 {
    for _ in 0..WARMUP_ITERATIONS {
        run(&mut fixture);
    }
    let start = Instant::now();
    for _ in 0..MEASURE_ITERATIONS {
        run(&mut fixture);
    }
    let elapsed = start.elapsed();
    black_box(fixture.checksum());
    elapsed.as_nanos() as f64 / MEASURE_ITERATIONS as f64
}

fn run_iteration(fixture: &mut GameplayFixture, candidate: IterationApi) {
    match candidate {
        IterationApi::ChunkClosure => fixture.iteration_closure(),
        IterationApi::ChunkFunction => fixture.iteration_function(),
    }
}

fn run_ai(fixture: &mut GameplayFixture, candidate: AiApi) {
    match candidate {
        AiApi::WorldGetPair => fixture.ai_world_get_pair(),
        AiApi::SplitAccessors => fixture.ai_split_accessors(),
        AiApi::PreparedEntityView => fixture.ai_prepared_entity_view(),
    }
}

fn run_position(fixture: &mut GameplayFixture, candidate: PositionApi) {
    match candidate {
        PositionApi::WorldGet => fixture.positions_world_get(),
        PositionApi::EntityAccessor => fixture.positions_accessor(),
        PositionApi::PreparedEntityView => fixture.positions_prepared_entity_view(),
    }
}

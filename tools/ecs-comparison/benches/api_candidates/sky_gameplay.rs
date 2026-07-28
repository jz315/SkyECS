use criterion::Criterion;
use serde::Serialize;
use sky_ecs_comparison::sky::{
    measure_ai_candidate, measure_frame_candidate, measure_iteration_candidate,
    measure_position_candidate, AiCandidate, FrameCandidateSelection, IterationCandidate,
    PositionCandidate,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const INITIAL_ROUNDS: usize = 4;
const EXTRA_ROUNDS: usize = 8;
const CLEAR_WIN: f64 = 0.98;
const CLEAR_LOSS: f64 = 1.02;
const FRAMES: f64 = 256.0;

trait Candidate: Copy + Eq {
    fn name(self) -> &'static str;
}

impl Candidate for IterationCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::Closure => "chunk closure",
            Self::Function => "chunk function",
        }
    }
}
impl Candidate for AiCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGetPair => "World get pair",
            Self::SplitAccessors => "split accessors",
            Self::PreparedEntityView => "PreparedEntityView tuple",
        }
    }
}
impl Candidate for PositionCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::WorldGet => "World get",
            Self::EntityAccessor => "EntityAccessor",
            Self::PreparedEntityAccessor => "PreparedEntityAccessor",
            Self::PreparedEntityView => "PreparedEntityView",
        }
    }
}

#[derive(Serialize)]
struct Round {
    first_ns: f64,
    second_ns: f64,
    second_ba_ns: f64,
    first_ba_ns: f64,
    ratio: f64,
}
#[derive(Serialize)]
struct Pair {
    first: String,
    second: String,
    rounds: Vec<Round>,
    decision: &'static str,
    winner: String,
    median_ratio: f64,
}
#[derive(Serialize)]
struct Phase {
    incumbent: String,
    comparisons: Vec<Pair>,
    condorcet: Option<String>,
    selected: String,
}
#[derive(Serialize)]
struct Metadata {
    source_revision: String,
    cpu: String,
    os: String,
    rustc: String,
    rustflags: String,
    profile: &'static str,
    command: &'static str,
}
#[derive(Serialize)]
struct Report {
    metadata: Metadata,
    contract: &'static str,
    iteration: Phase,
    ai: Phase,
    position: Phase,
    full_frame: Pair,
    accepted: bool,
    selected: String,
}

pub fn run(criterion: &mut Criterion) {
    if std::env::var_os("SKY_ECS_CERTIFY_GAMEPLAY_API").is_some() {
        certify();
        return;
    }
    let mut group = criterion.benchmark_group("api_candidates_sky_canonical_gameplay");
    for (name, candidate) in [
        ("iteration_closure", IterationCandidate::Closure),
        ("iteration_function", IterationCandidate::Function),
    ] {
        group.bench_function(name, |b| {
            b.iter_custom(|iterations| {
                repeat(iterations, || measure_iteration_candidate(candidate))
            })
        });
    }
    for (name, candidate) in [
        ("ai_world_pair", AiCandidate::WorldGetPair),
        ("ai_split_accessors", AiCandidate::SplitAccessors),
        ("ai_prepared_view", AiCandidate::PreparedEntityView),
    ] {
        group.bench_function(name, |b| {
            b.iter_custom(|iterations| repeat(iterations, || measure_ai_candidate(candidate)))
        });
    }
    for (name, candidate) in [
        ("position_world_get", PositionCandidate::WorldGet),
        ("position_accessor", PositionCandidate::EntityAccessor),
        (
            "position_prepared_accessor",
            PositionCandidate::PreparedEntityAccessor,
        ),
        (
            "position_prepared_view",
            PositionCandidate::PreparedEntityView,
        ),
    ] {
        group.bench_function(name, |b| {
            b.iter_custom(|iterations| repeat(iterations, || measure_position_candidate(candidate)))
        });
    }
    group.finish();
}

fn repeat(iterations: u64, mut measure: impl FnMut() -> Duration) -> Duration {
    (0..iterations).fold(Duration::ZERO, |sum, _| sum + measure())
}

fn certify() {
    let dirty = output("git", &["status", "--porcelain"]);
    assert!(
        dirty.is_empty(),
        "canonical certification requires a clean working tree"
    );
    let (iteration, iteration_winner) = phase(
        [IterationCandidate::Closure, IterationCandidate::Function],
        IterationCandidate::Closure,
        measure_iteration_candidate,
    );
    let (ai, ai_winner) = phase(
        [
            AiCandidate::WorldGetPair,
            AiCandidate::SplitAccessors,
            AiCandidate::PreparedEntityView,
        ],
        AiCandidate::PreparedEntityView,
        measure_ai_candidate,
    );
    let (position, position_winner) = phase(
        [
            PositionCandidate::WorldGet,
            PositionCandidate::EntityAccessor,
            PositionCandidate::PreparedEntityAccessor,
            PositionCandidate::PreparedEntityView,
        ],
        PositionCandidate::EntityAccessor,
        measure_position_candidate,
    );
    let incumbent = FrameCandidateSelection::PRODUCTION;
    let proposed = FrameCandidateSelection {
        iteration: iteration_winner,
        ai: ai_winner,
        position: position_winner,
    };
    let full_frame = compare(
        "production",
        incumbent,
        "proposed",
        proposed,
        measure_frame_candidate,
    );
    let accepted = proposed == incumbent
        || (full_frame.decision == "clear_2_percent_band" && full_frame.winner == "proposed");
    let selected = if accepted {
        selection_name(proposed)
    } else {
        selection_name(incumbent)
    };
    let report = Report {
        metadata: Metadata {
            source_revision: output("git", &["rev-parse", "HEAD"]),
            cpu: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".into()),
            os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            rustc: output("rustc", &["-Vv"]),
            rustflags: std::env::var("RUSTFLAGS").unwrap_or_default(),
            profile: "bench",
            command: "SKY_ECS_CERTIFY_GAMEPLAY_API=1 cargo bench -p sky_ecs_comparison --bench api_candidates --features api-experiments -- sky",
        },
        contract: "canonical 65,536-entity, 32+-shape, 256-frame five-phase gameplay trace; only the selected phase is timed",
        iteration, ai, position, full_frame, accepted, selected,
    };
    let path = std::env::var_os("SKY_ECS_CERTIFICATION_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/sky-ecs-canonical-gameplay-certification.json"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();
    println!("wrote {}", path.display());
}

fn phase<C: Candidate, const N: usize>(
    candidates: [C; N],
    incumbent: C,
    measure: fn(C) -> Duration,
) -> (Phase, C) {
    let mut comparisons = Vec::new();
    for left in 0..N {
        for right in left + 1..N {
            comparisons.push(compare(
                candidates[left].name(),
                candidates[left],
                candidates[right].name(),
                candidates[right],
                measure,
            ));
        }
    }
    let condorcet = candidates.iter().copied().find(|candidate| {
        comparisons
            .iter()
            .filter(|pair| pair.first == candidate.name() || pair.second == candidate.name())
            .all(|pair| pair.winner == candidate.name())
    });
    let selected = condorcet.unwrap_or(incumbent);
    (
        Phase {
            incumbent: incumbent.name().into(),
            comparisons,
            condorcet: condorcet.map(|value| value.name().into()),
            selected: selected.name().into(),
        },
        selected,
    )
}

fn compare<C: Copy>(
    first_name: &str,
    first: C,
    second_name: &str,
    second: C,
    measure: fn(C) -> Duration,
) -> Pair {
    let mut rounds = Vec::new();
    append(&mut rounds, INITIAL_ROUNDS, first, second, measure);
    if clear(&rounds).is_none() {
        append(&mut rounds, EXTRA_ROUNDS, first, second, measure);
    }
    let decision = clear(&rounds);
    let mut ratios: Vec<_> = rounds.iter().map(|round| round.ratio).collect();
    ratios.sort_by(f64::total_cmp);
    let median = if ratios.len() % 2 == 0 {
        (ratios[ratios.len() / 2 - 1] + ratios[ratios.len() / 2]) * 0.5
    } else {
        ratios[ratios.len() / 2]
    };
    let first_wins = decision.unwrap_or(median < 1.0);
    Pair {
        first: first_name.into(),
        second: second_name.into(),
        rounds,
        decision: if decision.is_some() {
            "clear_2_percent_band"
        } else {
            "order_neutral_median_fallback"
        },
        winner: if first_wins {
            first_name.into()
        } else {
            second_name.into()
        },
        median_ratio: median,
    }
}

fn append<C: Copy>(
    rounds: &mut Vec<Round>,
    count: usize,
    first: C,
    second: C,
    measure: fn(C) -> Duration,
) {
    for _ in 0..count {
        let a = per_frame(measure(first));
        let b = per_frame(measure(second));
        let b2 = per_frame(measure(second));
        let a2 = per_frame(measure(first));
        rounds.push(Round {
            first_ns: a,
            second_ns: b,
            second_ba_ns: b2,
            first_ba_ns: a2,
            ratio: ((a / b) * (a2 / b2)).sqrt(),
        });
    }
}
fn clear(rounds: &[Round]) -> Option<bool> {
    let mut ratios: Vec<_> = rounds.iter().map(|round| round.ratio).collect();
    ratios.sort_by(f64::total_cmp);
    let median = (ratios[(ratios.len() - 1) / 2] + ratios[ratios.len() / 2]) * 0.5;
    let first = rounds.iter().filter(|round| round.ratio < 1.0).count();
    let required = if rounds.len() == INITIAL_ROUNDS {
        INITIAL_ROUNDS
    } else {
        rounds.len() - 2
    };
    if first >= required && median < CLEAR_WIN {
        Some(true)
    } else if rounds.len() - first >= required && median > CLEAR_LOSS {
        Some(false)
    } else {
        None
    }
}
fn per_frame(duration: Duration) -> f64 {
    duration.as_nanos() as f64 / FRAMES
}
fn selection_name(value: FrameCandidateSelection) -> String {
    format!(
        "{:?} | {:?} | {:?}",
        value.iteration, value.ai, value.position
    )
}
fn output(program: &str, args: &[&str]) -> String {
    String::from_utf8(Command::new(program).args(args).output().unwrap().stdout)
        .unwrap()
        .trim()
        .into()
}

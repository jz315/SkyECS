use super::Candidate;
use serde::Serialize;
use sky_ecs_comparison::flecs_c::{
    measure_add_remove_candidate, measure_bulk_candidate, measure_spawn_candidate,
    AddRemoveCandidate, BulkConstructionCandidate, SpawnCandidate,
};
use sky_ecs_comparison::hecs::{measure_bulk_schema_candidate, BulkSchemaCandidate};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const INITIAL_ROUNDS: usize = 4;
const EXTRA_ROUNDS: usize = 8;
const CLEAR_WIN: f64 = 0.98;
const CLEAR_LOSS: f64 = 1.02;
const BULK_REPETITIONS: usize = 64;
const STRUCTURAL_REPETITIONS: usize = 128;

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
    repetitions_per_measurement: usize,
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
    working_tree_clean: bool,
    eligible_for_production: bool,
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
    contracts: [&'static str; 4],
    hecs_bulk_schema: Pair,
    flecs_bulk_construction: Phase,
    flecs_spawn_despawn: Phase,
    flecs_add_remove: Phase,
}

pub(super) fn run(require_clean: bool) {
    let clean = output("git", &["status", "--porcelain"]).is_empty();
    if require_clean {
        assert!(
            clean,
            "canonical structural API certification requires a clean working tree"
        );
    }

    let hecs_bulk_schema = compare(
        BulkSchemaCandidate::RebuildInTimedPath,
        BulkSchemaCandidate::PreparedInSetup,
        BULK_REPETITIONS,
        measure_hecs_bulk,
    );
    let flecs_bulk_construction = phase(
        [
            BulkConstructionCandidate::PreparedTable,
            BulkConstructionCandidate::ResolveTableFromIds,
            BulkConstructionCandidate::RemapInTimedPath,
        ],
        BulkConstructionCandidate::PreparedTable,
        BULK_REPETITIONS,
        measure_bulk_candidate,
    );
    let flecs_spawn_despawn = phase(
        [
            SpawnCandidate::TableGetMut,
            SpawnCandidate::TableSetId,
            SpawnCandidate::BulkOne,
        ],
        SpawnCandidate::TableGetMut,
        STRUCTURAL_REPETITIONS,
        measure_spawn_candidate,
    );
    let flecs_add_remove = phase(
        [
            AddRemoveCandidate::SetId,
            AddRemoveCandidate::Emplace,
            AddRemoveCandidate::AddThenGetMut,
        ],
        AddRemoveCandidate::Emplace,
        STRUCTURAL_REPETITIONS,
        measure_add_remove_candidate,
    );

    let command = if require_clean {
        "SKY_ECS_CERTIFY_STRUCTURAL_API=1 cargo bench -p sky_ecs_comparison --bench api_candidates --features api-experiments -- structural"
    } else {
        "SKY_ECS_DIAGNOSE_STRUCTURAL_API=1 cargo bench -p sky_ecs_comparison --bench api_candidates --features api-experiments -- structural"
    };
    let report = Report {
        metadata: Metadata {
            source_revision: output("git", &["rev-parse", "HEAD"]),
            working_tree_clean: clean,
            eligible_for_production: require_clean && clean,
            cpu: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".into()),
            os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            rustc: output("rustc", &["-Vv"]),
            rustflags: std::env::var("RUSTFLAGS").unwrap_or_default(),
            profile: "bench",
            command,
        },
        contracts: [
            "hecs bulk starts from neutral four-column Vecs and an empty schema-prepared World; batch allocation, writers, build and insertion are timed",
            "Flecs bulk starts from neutral four-column vectors and an empty World; candidates control static target-table preparation while descriptor construction and ecs_bulk_init are timed",
            "Flecs spawn/despawn performs 1,000 initialized two-component single-entity spawns followed by the canonical random deletion order",
            "Flecs add/remove writes Health(100) to 1,000 entities and removes it in the canonical independent orders",
        ],
        hecs_bulk_schema,
        flecs_bulk_construction,
        flecs_spawn_despawn,
        flecs_add_remove,
    };

    let path = std::env::var_os("SKY_ECS_STRUCTURAL_CERTIFICATION_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if require_clean {
                PathBuf::from("target/structural-api-certification.json")
            } else {
                PathBuf::from("target/structural-api-diagnostic.json")
            }
        });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create certification output directory");
    }
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("serialize certification")
        ),
    )
    .expect("write certification report");
    println!("wrote {}", path.display());
}

fn measure_hecs_bulk(candidate: BulkSchemaCandidate, repetitions: usize) -> Duration {
    (0..repetitions).fold(Duration::ZERO, |sum, _| {
        sum + measure_bulk_schema_candidate(candidate)
    })
}

fn phase<C: Candidate, F: Copy + Fn(C, usize) -> Duration, const N: usize>(
    candidates: [C; N],
    incumbent: C,
    repetitions: usize,
    measure: F,
) -> Phase {
    let mut comparisons = Vec::new();
    for left in 0..N {
        for right in left + 1..N {
            comparisons.push(compare(
                candidates[left],
                candidates[right],
                repetitions,
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
    Phase {
        incumbent: incumbent.name().into(),
        comparisons,
        condorcet: condorcet.map(|value| value.name().into()),
        selected: selected.name().into(),
    }
}

fn compare<C: Candidate, F: Copy + Fn(C, usize) -> Duration>(
    first: C,
    second: C,
    repetitions: usize,
    measure: F,
) -> Pair {
    let mut rounds = Vec::new();
    append(
        &mut rounds,
        INITIAL_ROUNDS,
        first,
        second,
        repetitions,
        measure,
    );
    if clear(&rounds).is_none() {
        append(
            &mut rounds,
            EXTRA_ROUNDS,
            first,
            second,
            repetitions,
            measure,
        );
    }

    let decision = clear(&rounds);
    let median_ratio = median(rounds.iter().map(|round| round.ratio).collect());
    let first_wins = decision.unwrap_or(median_ratio < 1.0);
    Pair {
        first: first.name().into(),
        second: second.name().into(),
        repetitions_per_measurement: repetitions,
        rounds,
        decision: if decision.is_some() {
            "clear_2_percent_band"
        } else {
            "order_neutral_median_fallback"
        },
        winner: if first_wins {
            first.name().into()
        } else {
            second.name().into()
        },
        median_ratio,
    }
}

fn append<C: Candidate, F: Copy + Fn(C, usize) -> Duration>(
    rounds: &mut Vec<Round>,
    count: usize,
    first: C,
    second: C,
    repetitions: usize,
    measure: F,
) {
    for _ in 0..count {
        let first_ns = per_operation(measure(first, repetitions), repetitions);
        let second_ns = per_operation(measure(second, repetitions), repetitions);
        let second_ba_ns = per_operation(measure(second, repetitions), repetitions);
        let first_ba_ns = per_operation(measure(first, repetitions), repetitions);
        rounds.push(Round {
            first_ns,
            second_ns,
            second_ba_ns,
            first_ba_ns,
            ratio: ((first_ns / second_ns) * (first_ba_ns / second_ba_ns)).sqrt(),
        });
    }
}

fn clear(rounds: &[Round]) -> Option<bool> {
    let median_ratio = median(rounds.iter().map(|round| round.ratio).collect());
    let first_wins = rounds.iter().filter(|round| round.ratio < 1.0).count();
    let required = if rounds.len() == INITIAL_ROUNDS {
        INITIAL_ROUNDS
    } else {
        rounds.len() - 2
    };
    if first_wins >= required && median_ratio < CLEAR_WIN {
        Some(true)
    } else if rounds.len() - first_wins >= required && median_ratio > CLEAR_LOSS {
        Some(false)
    } else {
        None
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    (values[(values.len() - 1) / 2] + values[values.len() / 2]) * 0.5
}

fn per_operation(duration: Duration, repetitions: usize) -> f64 {
    duration.as_nanos() as f64 / repetitions as f64
}

fn output(program: &str, args: &[&str]) -> String {
    String::from_utf8(
        Command::new(program)
            .args(args)
            .output()
            .expect("run metadata command")
            .stdout,
    )
    .expect("metadata command output is UTF-8")
    .trim()
    .into()
}

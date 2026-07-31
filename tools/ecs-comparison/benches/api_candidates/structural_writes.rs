use criterion::Criterion;
use sky_ecs_comparison::flecs_c::{
    measure_add_remove_candidate, measure_bulk_candidate, measure_spawn_candidate,
    AddRemoveCandidate, BulkConstructionCandidate, SpawnCandidate,
};
use sky_ecs_comparison::hecs::{measure_bulk_schema_candidate, BulkSchemaCandidate};
use std::time::Duration;

#[path = "structural_writes/certification.rs"]
mod certification;

pub(crate) trait Candidate: Copy + Eq {
    fn name(self) -> &'static str;
}

impl Candidate for BulkSchemaCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::RebuildInTimedPath => "rebuild schema in timed path",
            Self::PreparedInSetup => "prepared schema in setup",
        }
    }
}

impl Candidate for BulkConstructionCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::PreparedTable => "prepared table and component mapping",
            Self::ResolveTableFromIds => "resolve table from IDs in timed path",
            Self::RemapInTimedPath => "prepared table + remap components in timed path",
        }
    }
}

impl Candidate for SpawnCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::TableGetMut => "table create + get_mut",
            Self::TableSetId => "table create + set_id",
            Self::BulkOne => "bulk_init count one",
        }
    }
}

impl Candidate for AddRemoveCandidate {
    fn name(self) -> &'static str {
        match self {
            Self::SetId => "set_id + remove_id",
            Self::Emplace => "emplace + remove_id",
            Self::AddThenGetMut => "add_id + get_mut + remove_id",
        }
    }
}

pub fn run(criterion: &mut Criterion) {
    if std::env::var_os("SKY_ECS_CERTIFY_STRUCTURAL_API").is_some() {
        certification::run(true);
        return;
    }
    if std::env::var_os("SKY_ECS_DIAGNOSE_STRUCTURAL_API").is_some() {
        certification::run(false);
        return;
    }

    let mut group = criterion.benchmark_group("api_candidates_structural_writes");
    for candidate in [
        BulkSchemaCandidate::RebuildInTimedPath,
        BulkSchemaCandidate::PreparedInSetup,
    ] {
        group.bench_function(format!("hecs_bulk/{}", candidate.name()), |bencher| {
            bencher.iter_custom(|iterations| {
                repeat(iterations, || measure_bulk_schema_candidate(candidate))
            });
        });
    }
    for candidate in [
        BulkConstructionCandidate::PreparedTable,
        BulkConstructionCandidate::ResolveTableFromIds,
        BulkConstructionCandidate::RemapInTimedPath,
    ] {
        group.bench_function(format!("flecs_bulk/{}", candidate.name()), |bencher| {
            bencher
                .iter_custom(|iterations| measure_bulk_candidate(candidate, to_usize(iterations)));
        });
    }
    for candidate in [
        SpawnCandidate::TableGetMut,
        SpawnCandidate::TableSetId,
        SpawnCandidate::BulkOne,
    ] {
        group.bench_function(format!("flecs_spawn/{}", candidate.name()), |bencher| {
            bencher
                .iter_custom(|iterations| measure_spawn_candidate(candidate, to_usize(iterations)));
        });
    }
    for candidate in [
        AddRemoveCandidate::SetId,
        AddRemoveCandidate::Emplace,
        AddRemoveCandidate::AddThenGetMut,
    ] {
        group.bench_function(
            format!("flecs_add_remove/{}", candidate.name()),
            |bencher| {
                bencher.iter_custom(|iterations| {
                    measure_add_remove_candidate(candidate, to_usize(iterations))
                });
            },
        );
    }
    group.finish();
}

fn repeat(iterations: u64, mut measure: impl FnMut() -> Duration) -> Duration {
    (0..iterations).fold(Duration::ZERO, |sum, _| sum + measure())
}

fn to_usize(iterations: u64) -> usize {
    usize::try_from(iterations).expect("Criterion iteration count exceeds usize")
}

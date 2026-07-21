use crate::common::{suite_bundle, PositionComponent, VelocityComponent};
use hecs::{Archetype, PreparedQuery, World};
use std::hint::black_box;
use std::time::{Duration, Instant};

type MovementQuery = (&'static mut PositionComponent, &'static VelocityComponent);

#[derive(Debug, serde::Serialize)]
pub struct HecsDenseCandidateResult {
    pub entity_count: usize,
    pub rotations: usize,
    pub traversals_per_rotation: usize,
    pub first: &'static str,
    pub first_ns_per_traversal: f64,
    pub second: &'static str,
    pub second_ns_per_traversal: f64,
    pub winner: &'static str,
    pub difference_percent: f64,
    pub paired_ratios: Vec<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct HecsDenseApiCertification {
    pub prepared_shared_vs_batched: Vec<HecsDenseCandidateResult>,
    pub prepared_shared_vs_columns: Vec<HecsDenseCandidateResult>,
    pub batched_vs_columns: Vec<HecsDenseCandidateResult>,
}

pub fn certify_dense_apis(rotations: usize) -> HecsDenseApiCertification {
    assert!(rotations > 0);
    let workloads = [(10_000, 8_192), (100_000, 1_024), (1_000_000, 64)];
    HecsDenseApiCertification {
        prepared_shared_vs_batched: workloads
            .into_iter()
            .map(|(entity_count, traversals)| {
                certify_prepared_shared_vs_batched(entity_count, rotations, traversals)
            })
            .collect(),
        prepared_shared_vs_columns: workloads
            .into_iter()
            .map(|(entity_count, traversals)| {
                certify_prepared_shared_vs_columns(entity_count, rotations, traversals)
            })
            .collect(),
        batched_vs_columns: workloads
            .into_iter()
            .map(|(entity_count, traversals)| {
                certify_batched_vs_columns(entity_count, rotations, traversals)
            })
            .collect(),
    }
}

fn world_with_entities(entity_count: usize) -> World {
    let mut world = World::new();
    world.spawn_batch((0..entity_count).map(|_| suite_bundle()));
    world
}

fn certify_prepared_shared_vs_batched(
    entity_count: usize,
    rotations: usize,
    traversals_per_rotation: usize,
) -> HecsDenseCandidateResult {
    let shared_world = world_with_entities(entity_count);
    let mut shared_query = PreparedQuery::<MovementQuery>::default();
    assert_eq!(
        shared_query.query(&shared_world).iter().count(),
        entity_count
    );
    let mut shared = || run_prepared_shared(&shared_world, &mut shared_query);

    let mut batched_world = world_with_entities(entity_count);
    assert_eq!(
        batched_world
            .query_mut::<MovementQuery>()
            .into_iter_batched(u32::MAX)
            .map(Iterator::count)
            .sum::<usize>(),
        entity_count
    );
    let mut batched = || run_batched(&mut batched_world);

    paired_certification(
        entity_count,
        rotations,
        traversals_per_rotation,
        "PreparedQuery::query().iter()",
        &mut shared,
        "World::query_mut().into_iter_batched()",
        &mut batched,
    )
}

fn certify_prepared_shared_vs_columns(
    entity_count: usize,
    rotations: usize,
    traversals_per_rotation: usize,
) -> HecsDenseCandidateResult {
    let shared_world = world_with_entities(entity_count);
    let mut shared_query = PreparedQuery::<MovementQuery>::default();
    assert_eq!(
        shared_query.query(&shared_world).iter().count(),
        entity_count
    );
    let mut shared = || run_prepared_shared(&shared_world, &mut shared_query);

    let columns_world = world_with_entities(entity_count);
    let archetypes = matching_archetypes(&columns_world, entity_count);
    let mut columns = || run_archetype_columns(&archetypes);

    paired_certification(
        entity_count,
        rotations,
        traversals_per_rotation,
        "PreparedQuery::query().iter()",
        &mut shared,
        "prepared Archetype::get columns",
        &mut columns,
    )
}

fn certify_batched_vs_columns(
    entity_count: usize,
    rotations: usize,
    traversals_per_rotation: usize,
) -> HecsDenseCandidateResult {
    let mut batched_world = world_with_entities(entity_count);
    assert_eq!(
        batched_world
            .query_mut::<MovementQuery>()
            .into_iter_batched(u32::MAX)
            .map(Iterator::count)
            .sum::<usize>(),
        entity_count
    );
    let mut batched = || run_batched(&mut batched_world);

    let columns_world = world_with_entities(entity_count);
    let archetypes = matching_archetypes(&columns_world, entity_count);
    let mut columns = || run_archetype_columns(&archetypes);

    paired_certification(
        entity_count,
        rotations,
        traversals_per_rotation,
        "World::query_mut().into_iter_batched()",
        &mut batched,
        "prepared Archetype::get columns",
        &mut columns,
    )
}

#[allow(clippy::too_many_arguments)]
fn paired_certification(
    entity_count: usize,
    rotations: usize,
    traversals_per_rotation: usize,
    first_name: &'static str,
    first: &mut impl FnMut(),
    second_name: &'static str,
    second: &mut impl FnMut(),
) -> HecsDenseCandidateResult {
    for _ in 0..2 {
        first();
        second();
    }

    let mut first_elapsed = Duration::ZERO;
    let mut second_elapsed = Duration::ZERO;
    let mut paired_ratios = Vec::with_capacity(rotations);
    for rotation in 0..rotations {
        let (first_duration, second_duration) = if rotation % 2 == 0 {
            (
                time_traversals(first, traversals_per_rotation),
                time_traversals(second, traversals_per_rotation),
            )
        } else {
            let second_duration = time_traversals(second, traversals_per_rotation);
            let first_duration = time_traversals(first, traversals_per_rotation);
            (first_duration, second_duration)
        };
        first_elapsed += first_duration;
        second_elapsed += second_duration;
        paired_ratios.push(second_duration.as_secs_f64() / first_duration.as_secs_f64());
    }

    let traversal_count = (rotations * traversals_per_rotation) as f64;
    let first_ns = first_elapsed.as_nanos() as f64 / traversal_count;
    let second_ns = second_elapsed.as_nanos() as f64 / traversal_count;
    let (winner, faster, slower) = if first_ns <= second_ns {
        (first_name, first_ns, second_ns)
    } else {
        (second_name, second_ns, first_ns)
    };
    HecsDenseCandidateResult {
        entity_count,
        rotations,
        traversals_per_rotation,
        first: first_name,
        first_ns_per_traversal: first_ns,
        second: second_name,
        second_ns_per_traversal: second_ns,
        winner,
        difference_percent: (slower / faster - 1.0) * 100.0,
        paired_ratios,
    }
}

fn time_traversals(candidate: &mut impl FnMut(), traversals: usize) -> Duration {
    let start = Instant::now();
    for _ in 0..traversals {
        candidate();
    }
    start.elapsed()
}

fn run_prepared_shared(world: &World, query: &mut PreparedQuery<MovementQuery>) {
    for (position, velocity) in query.query(world).iter() {
        position.0 += velocity.0;
    }
    black_box(world);
}

fn run_batched(world: &mut World) {
    for batch in world
        .query_mut::<MovementQuery>()
        .into_iter_batched(u32::MAX)
    {
        for (position, velocity) in batch {
            position.0 += velocity.0;
        }
    }
    black_box(world);
}

fn matching_archetypes(world: &World, expected: usize) -> Vec<&Archetype> {
    let archetypes = world
        .archetypes()
        .filter(|archetype| archetype.satisfies::<MovementQuery>())
        .collect::<Vec<_>>();
    assert_eq!(
        archetypes
            .iter()
            .map(|archetype| archetype.len() as usize)
            .sum::<usize>(),
        expected
    );
    archetypes
}

fn run_archetype_columns(archetypes: &[&Archetype]) {
    for archetype in archetypes {
        let mut positions = archetype
            .get::<&mut PositionComponent>()
            .expect("matching archetype must contain PositionComponent");
        let velocities = archetype
            .get::<&VelocityComponent>()
            .expect("matching archetype must contain VelocityComponent");
        update_columns(&mut positions, &velocities);
    }
    black_box(archetypes);
}

#[inline(never)]
fn update_columns(positions: &mut [PositionComponent], velocities: &[VelocityComponent]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0 += velocity.0;
    }
}

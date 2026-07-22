use super::{random_fragment_masks, random_fragment_match_count};
use crate::Engine;

const ENGINES_WITHOUT_FREECS: &[Engine] = &[
    Engine::Sky,
    Engine::Hecs,
    Engine::Bevy,
    Engine::FlecsC,
    Engine::Shipyard,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkClass {
    Comparable,
    Scenario,
    Diagnostic,
}

impl BenchmarkClass {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Comparable => "comparable",
            Self::Scenario => "scenario",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkItems {
    None,
    Fixed(usize),
    RandomFragment {
        component_count: usize,
        term_count: usize,
    },
}

impl WorkItems {
    pub fn resolve(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Fixed(count) => Some(count),
            Self::RandomFragment {
                component_count,
                term_count,
            } => Some(random_fragment_match_count(
                &random_fragment_masks(component_count),
                term_count,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkSpec {
    pub family: &'static str,
    pub class: BenchmarkClass,
    pub work_items: WorkItems,
    pub engines: &'static [Engine],
}

const fn fixed(family: &'static str, count: usize) -> BenchmarkSpec {
    BenchmarkSpec {
        family,
        class: BenchmarkClass::Comparable,
        work_items: WorkItems::Fixed(count),
        engines: &Engine::ALL,
    }
}

const fn fixed_class(family: &'static str, count: usize, class: BenchmarkClass) -> BenchmarkSpec {
    BenchmarkSpec {
        family,
        class,
        work_items: WorkItems::Fixed(count),
        engines: &Engine::ALL,
    }
}

const fn no_items(family: &'static str, class: BenchmarkClass) -> BenchmarkSpec {
    BenchmarkSpec {
        family,
        class,
        work_items: WorkItems::None,
        engines: &Engine::ALL,
    }
}

const fn random(family: &'static str, component_count: usize, term_count: usize) -> BenchmarkSpec {
    BenchmarkSpec {
        family,
        class: BenchmarkClass::Comparable,
        work_items: WorkItems::RandomFragment {
            component_count,
            term_count,
        },
        engines: &Engine::ALL,
    }
}

const fn random_without_freecs(
    family: &'static str,
    component_count: usize,
    term_count: usize,
) -> BenchmarkSpec {
    BenchmarkSpec {
        family,
        class: BenchmarkClass::Comparable,
        work_items: WorkItems::RandomFragment {
            component_count,
            term_count,
        },
        engines: ENGINES_WITHOUT_FREECS,
    }
}

pub const CANONICAL_BENCHMARKS: [BenchmarkSpec; 44] = [
    fixed_class(
        "scenario_native_bulk_construction/insert_10k",
        10_000,
        BenchmarkClass::Scenario,
    ),
    fixed("prepared_construction/single_insert_10k", 10_000),
    fixed("prepared_iteration/simple_10k", 10_000),
    fixed("prepared_iteration_large/simple_100k", 100_000),
    fixed("prepared_iteration_1m/simple_1m", 1_000_000),
    fixed("prepared_fragmented_iteration/fragmented_26x400", 10_400),
    random(
        "prepared_random_fragmented_iteration/random_6_tags_1_term",
        6,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_6_tags_4_terms",
        6,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_8_tags_1_term",
        8,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_8_tags_4_terms",
        8,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_tags_1_term",
        10,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_tags_4_terms",
        10,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_tags_8_terms",
        10,
        8,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_tags_1_term",
        16,
        1,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_tags_4_terms",
        16,
        4,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_tags_8_terms",
        16,
        8,
    ),
    random(
        "prepared_random_fragmented_iteration/random_6_components_1_term",
        6,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_6_components_4_terms",
        6,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_8_components_1_term",
        8,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_8_components_4_terms",
        8,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_components_1_term",
        10,
        1,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_components_4_terms",
        10,
        4,
    ),
    random(
        "prepared_random_fragmented_iteration/random_10_components_8_terms",
        10,
        8,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_components_1_term",
        16,
        1,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_components_4_terms",
        16,
        4,
    ),
    random_without_freecs(
        "prepared_random_fragmented_iteration/random_16_components_8_terms",
        16,
        8,
    ),
    no_items("diagnostic_heavy_compute/heavy", BenchmarkClass::Diagnostic),
    fixed("entity_id_random_access/hot_10k", 10_000),
    fixed("entity_id_random_access/warm_100k", 100_000),
    fixed_class(
        "scenario_fixed_sequence_access/build_10k",
        10_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/steady_10k",
        10_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_10k_x1",
        10_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_10k_x4",
        40_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_10k_x16",
        160_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_10k_x64",
        640_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/build_100k",
        100_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/steady_100k",
        100_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_100k_x1",
        100_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_100k_x4",
        400_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_100k_x16",
        1_600_000,
        BenchmarkClass::Scenario,
    ),
    fixed_class(
        "scenario_fixed_sequence_access/amortized_100k_x64",
        6_400_000,
        BenchmarkClass::Scenario,
    ),
    fixed("entity_ops/spawn_despawn_1k", 1_000),
    fixed("entity_ops/add_remove_component_1k", 1_000),
    no_items("scenario_gameplay_frame/frame", BenchmarkClass::Scenario),
];

pub const GAMEPLAY_PHASE_BENCHMARKS: [BenchmarkSpec; 5] = [
    no_items(
        "diagnostic_gameplay_phases/iteration",
        BenchmarkClass::Diagnostic,
    ),
    no_items(
        "diagnostic_gameplay_phases/ai_source_lookup",
        BenchmarkClass::Diagnostic,
    ),
    no_items(
        "diagnostic_gameplay_phases/target_position_lookup",
        BenchmarkClass::Diagnostic,
    ),
    no_items(
        "diagnostic_gameplay_phases/status_transition",
        BenchmarkClass::Diagnostic,
    ),
    no_items(
        "diagnostic_gameplay_phases/projectile_recycle",
        BenchmarkClass::Diagnostic,
    ),
];

pub fn benchmark_spec(family: &str) -> Option<&'static BenchmarkSpec> {
    CANONICAL_BENCHMARKS
        .iter()
        .chain(GAMEPLAY_PHASE_BENCHMARKS.iter())
        .find(|spec| spec.family == family)
}

pub fn benchmark_work_items(full_id: &str) -> Option<usize> {
    let (family, _) = full_id.rsplit_once('/')?;
    benchmark_spec(family)?.work_items.resolve()
}

pub fn benchmark_class(full_id: &str) -> Option<BenchmarkClass> {
    let (family, _) = full_id.rsplit_once('/')?;
    Some(benchmark_spec(family)?.class)
}

pub fn fixed_sequence_plan_payload_bytes(full_id: &str) -> Option<usize> {
    let (family, _) = full_id.rsplit_once('/')?;
    if !family.starts_with("scenario_fixed_sequence_access/") {
        return None;
    }
    let entity_count: usize = if family.contains("100k") {
        100_000
    } else if family.contains("10k") {
        10_000
    } else {
        return None;
    };
    entity_count.checked_mul(std::mem::size_of::<*const ()>())
}

pub fn fixed_sequence_amortized_traversals(full_id: &str) -> Option<usize> {
    let (family, _) = full_id.rsplit_once('/')?;
    let suffix = family
        .strip_prefix("scenario_fixed_sequence_access/amortized_")?
        .rsplit_once("_x")?
        .1;
    suffix.parse().ok()
}

pub fn is_canonical_group(group: &str) -> bool {
    CANONICAL_BENCHMARKS
        .iter()
        .chain(GAMEPLAY_PHASE_BENCHMARKS.iter())
        .any(|spec| {
            spec.family
                .split_once('/')
                .is_some_and(|(candidate, _)| candidate == group)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_has_unique_families() {
        let families: BTreeSet<_> = CANONICAL_BENCHMARKS
            .iter()
            .map(|spec| spec.family)
            .collect();
        assert_eq!(families.len(), CANONICAL_BENCHMARKS.len());
    }

    #[test]
    fn catalog_resolves_random_fragment_work_items() {
        assert_eq!(
            benchmark_work_items(
                "prepared_random_fragmented_iteration/random_16_components_4_terms/sky"
            ),
            Some(4_103)
        );
    }

    #[test]
    fn catalog_classifies_every_family() {
        assert_eq!(
            CANONICAL_BENCHMARKS
                .iter()
                .filter(|spec| spec.class == BenchmarkClass::Comparable)
                .count(),
            29
        );
        assert_eq!(
            CANONICAL_BENCHMARKS
                .iter()
                .filter(|spec| spec.class == BenchmarkClass::Scenario)
                .count(),
            14
        );
        assert_eq!(
            CANONICAL_BENCHMARKS
                .iter()
                .filter(|spec| spec.class == BenchmarkClass::Diagnostic)
                .count(),
            1
        );
        assert_eq!(
            benchmark_class("scenario_gameplay_frame/frame/sky"),
            Some(BenchmarkClass::Scenario)
        );
    }

    #[test]
    fn catalog_excludes_freecs_only_from_the_pathological_16_component_matrix() {
        let sixteen =
            benchmark_spec("prepared_random_fragmented_iteration/random_16_tags_1_term").unwrap();
        assert!(!sixteen.engines.contains(&Engine::Freecs));
        assert_eq!(sixteen.engines.len(), 5);

        let ten =
            benchmark_spec("prepared_random_fragmented_iteration/random_10_tags_1_term").unwrap();
        assert!(ten.engines.contains(&Engine::Freecs));
        assert_eq!(ten.engines, &Engine::ALL);
    }

    #[test]
    fn fixed_sequence_metadata_reports_plan_and_amortization_costs() {
        assert_eq!(
            fixed_sequence_plan_payload_bytes("scenario_fixed_sequence_access/steady_10k/sky"),
            Some(10_000 * std::mem::size_of::<*const ()>())
        );
        assert_eq!(
            fixed_sequence_plan_payload_bytes("entity_id_random_access/hot_10k/sky"),
            None
        );
        assert_eq!(
            fixed_sequence_amortized_traversals(
                "scenario_fixed_sequence_access/amortized_100k_x64/hecs"
            ),
            Some(64)
        );
        assert_eq!(
            fixed_sequence_amortized_traversals("scenario_fixed_sequence_access/steady_100k/hecs"),
            None
        );
    }
}

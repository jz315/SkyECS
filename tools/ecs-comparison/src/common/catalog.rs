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

pub const CANONICAL_BENCHMARKS: [BenchmarkSpec; 32] = [
    fixed("prepared_construction/native_bulk_insert_10k", 10_000),
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
    fixed("prepared_random_access/hot_10k", 10_000),
    fixed("prepared_random_access/warm_100k", 100_000),
    fixed("entity_ops/spawn_despawn_1k", 1_000),
    fixed("entity_ops/add_remove_component_1k", 1_000),
    no_items("scenario_gameplay_frame/frame", BenchmarkClass::Scenario),
];

pub fn benchmark_spec(family: &str) -> Option<&'static BenchmarkSpec> {
    CANONICAL_BENCHMARKS
        .iter()
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

pub fn is_canonical_group(group: &str) -> bool {
    CANONICAL_BENCHMARKS.iter().any(|spec| {
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
            30
        );
        assert_eq!(
            CANONICAL_BENCHMARKS
                .iter()
                .filter(|spec| spec.class == BenchmarkClass::Scenario)
                .count(),
            1
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
}

use super::model::{OrderBias, Summary};
use sky_ecs_comparison::Engine;
use std::collections::BTreeMap;
use std::io::{self, Write};

#[derive(Clone, Copy)]
struct DisplayRow {
    family: &'static str,
    test: &'static str,
    variant: &'static str,
}

const COMPARABLE_ROWS: [DisplayRow; 10] = [
    DisplayRow {
        family: "entity_construction/single_insert_10k",
        test: "Entity construction",
        variant: "Individual 10K",
    },
    DisplayRow {
        family: "entity_construction/insert_10k",
        test: "Entity construction",
        variant: "Native bulk 10K",
    },
    DisplayRow {
        family: "entity_ops/spawn_despawn_1k",
        test: "Entity operations",
        variant: "Spawn/despawn 1K",
    },
    DisplayRow {
        family: "entity_ops/add_remove_component_1k",
        test: "Entity operations",
        variant: "Add/remove component 1K",
    },
    DisplayRow {
        family: "entity_id_random_access/hot_10k",
        test: "EntityId random access",
        variant: "Hot 10K",
    },
    DisplayRow {
        family: "entity_id_random_access/warm_100k",
        test: "EntityId random access",
        variant: "Warm 100K",
    },
    DisplayRow {
        family: "prepared_iteration/simple_10k",
        test: "Prepared iteration",
        variant: "10K",
    },
    DisplayRow {
        family: "prepared_iteration_large/simple_100k",
        test: "Prepared iteration",
        variant: "100K",
    },
    DisplayRow {
        family: "prepared_iteration_1m/simple_1m",
        test: "Prepared iteration",
        variant: "1M",
    },
    DisplayRow {
        family: "prepared_fragmented_iteration/fragmented_26x400",
        test: "Fragmented iteration",
        variant: "26 × 400",
    },
];

const RANDOM_TAG_ROWS: [DisplayRow; 10] = [
    random_row("random_fragmentation/random_6_tags_1_term", "6 Tags", "1"),
    random_row("random_fragmentation/random_6_tags_4_terms", "6 Tags", "4"),
    random_row("random_fragmentation/random_8_tags_1_term", "8 Tags", "1"),
    random_row("random_fragmentation/random_8_tags_4_terms", "8 Tags", "4"),
    random_row("random_fragmentation/random_10_tags_1_term", "10 Tags", "1"),
    random_row(
        "random_fragmentation/random_10_tags_4_terms",
        "10 Tags",
        "4",
    ),
    random_row(
        "random_fragmentation/random_10_tags_8_terms",
        "10 Tags",
        "8",
    ),
    random_row("random_fragmentation/random_16_tags_1_term", "16 Tags", "1"),
    random_row(
        "random_fragmentation/random_16_tags_4_terms",
        "16 Tags",
        "4",
    ),
    random_row(
        "random_fragmentation/random_16_tags_8_terms",
        "16 Tags",
        "8",
    ),
];

const RANDOM_COMPONENT_ROWS: [DisplayRow; 10] = [
    random_row(
        "random_fragmentation/random_6_components_1_term",
        "6 Components",
        "1",
    ),
    random_row(
        "random_fragmentation/random_6_components_4_terms",
        "6 Components",
        "4",
    ),
    random_row(
        "random_fragmentation/random_8_components_1_term",
        "8 Components",
        "1",
    ),
    random_row(
        "random_fragmentation/random_8_components_4_terms",
        "8 Components",
        "4",
    ),
    random_row(
        "random_fragmentation/random_10_components_1_term",
        "10 Components",
        "1",
    ),
    random_row(
        "random_fragmentation/random_10_components_4_terms",
        "10 Components",
        "4",
    ),
    random_row(
        "random_fragmentation/random_10_components_8_terms",
        "10 Components",
        "8",
    ),
    random_row(
        "random_fragmentation/random_16_components_1_term",
        "16 Components",
        "1",
    ),
    random_row(
        "random_fragmentation/random_16_components_4_terms",
        "16 Components",
        "4",
    ),
    random_row(
        "random_fragmentation/random_16_components_8_terms",
        "16 Components",
        "8",
    ),
];

const GAMEPLAY_ROWS: [DisplayRow; 6] = [
    single_column_row("gameplay_scenario/frame", "Full frame"),
    single_column_row("gameplay_scenario/iteration", "Iteration"),
    single_column_row("gameplay_scenario/ai_source_lookup", "AI source lookup"),
    single_column_row(
        "gameplay_scenario/target_position_lookup",
        "Target Position lookup",
    ),
    single_column_row("gameplay_scenario/status_transition", "Status transition"),
    single_column_row("gameplay_scenario/projectile_recycle", "Projectile recycle"),
];

const DIAGNOSTIC_ROWS: [DisplayRow; 1] = [single_column_row(
    "diagnostic_heavy_compute/heavy",
    "Heavy compute",
)];

const fn random_row(family: &'static str, shapes: &'static str, terms: &'static str) -> DisplayRow {
    DisplayRow {
        family,
        test: shapes,
        variant: terms,
    }
}

const fn single_column_row(family: &'static str, label: &'static str) -> DisplayRow {
    DisplayRow {
        family,
        test: label,
        variant: "",
    }
}

struct SummaryIndex<'a> {
    by_family_and_engine: BTreeMap<(&'a str, &'a str), &'a Summary>,
}

impl<'a> SummaryIndex<'a> {
    fn new(summaries: &'a [Summary]) -> Self {
        let by_family_and_engine = summaries
            .iter()
            .filter_map(|summary| {
                let (family, engine) = summary.benchmark.rsplit_once('/')?;
                Some(((family, engine), summary))
            })
            .collect();
        Self {
            by_family_and_engine,
        }
    }

    fn cell(&self, family: &str, engine: Engine) -> String {
        let Some(summary) = self
            .by_family_and_engine
            .get(&(family, engine.name()))
            .copied()
        else {
            return "N/A".to_owned();
        };
        let minimum = Engine::ALL
            .iter()
            .filter_map(|candidate| {
                self.by_family_and_engine
                    .get(&(family, candidate.name()))
                    .map(|candidate| candidate.median_ns)
            })
            .fold(f64::INFINITY, f64::min);
        let value = format_duration(summary.median_ns);
        let mut rendered = if summary.median_ns == minimum {
            format!("**{value}**")
        } else {
            value
        };
        if summary.noisy {
            rendered.push('†');
        }
        rendered
    }
}

pub(super) fn write_markdown(
    output: &mut impl Write,
    source: Option<&str>,
    summaries: &[Summary],
    order_bias: &OrderBias,
    allow_dirty: bool,
) -> io::Result<()> {
    if allow_dirty {
        writeln!(output, "# NON-PUBLICATION / DIRTY WORKTREE\n")?;
    } else {
        writeln!(output, "# Compare-ECS benchmark results\n")?;
    }
    match source {
        Some(source) => writeln!(output, "{source}\n")?,
        None => writeln!(output, "Local run\n")?,
    }

    let index = SummaryIndex::new(summaries);
    writeln!(output, "## Comparable\n")?;
    write_two_column_table(output, &index, "Test", "Scale / Mode", &COMPARABLE_ROWS)?;

    writeln!(output, "## Random Fragmentation\n")?;
    writeln!(
        output,
        "This section follows Sander Mertens' public random-fragmentation benchmark.\n"
    )?;
    writeln!(output, "### Tags\n")?;
    write_two_column_table(output, &index, "Shapes", "Terms", &RANDOM_TAG_ROWS)?;
    writeln!(output, "### Data Components\n")?;
    write_two_column_table(output, &index, "Shapes", "Terms", &RANDOM_COMPONENT_ROWS)?;

    writeln!(output, "## Gameplay Scenario\n")?;
    write_single_column_table(output, &index, "Gameplay item", &GAMEPLAY_ROWS)?;

    writeln!(output, "## Diagnostic\n")?;
    write_single_column_table(output, &index, "Diagnostic", &DIAGNOSTIC_ROWS)?;

    writeln!(output, "## Notes\n")?;
    writeln!(
        output,
        "Lower is faster. **Bold** marks the lowest median in a row; `†` marks a noisy cell; `N/A` means no result."
    )?;
    if order_bias.available {
        writeln!(
            output,
            "\nOrder bias: maximum deviation {:.2}%; position spread {:.2}%.",
            order_bias.max_deviation_percent.unwrap_or_default(),
            order_bias.spread_percent.unwrap_or_default()
        )?;
    } else {
        writeln!(
            output,
            "\nOrder bias: **N/A.** {}.",
            order_bias
                .reason
                .as_deref()
                .unwrap_or("complete position-balanced data is unavailable")
        )?;
    }
    Ok(())
}

fn write_two_column_table(
    output: &mut impl Write,
    index: &SummaryIndex<'_>,
    first_header: &str,
    second_header: &str,
    rows: &[DisplayRow],
) -> io::Result<()> {
    write!(
        output,
        "| {first_header} | {second_header} | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |\n\
         |---|---|---:|---:|---:|---:|---:|---:|\n"
    )?;
    for row in rows {
        write!(output, "| {} | {} |", row.test, row.variant)?;
        for engine in Engine::ALL {
            write!(output, " {} |", index.cell(row.family, engine))?;
        }
        writeln!(output)?;
    }
    writeln!(output)
}

fn write_single_column_table(
    output: &mut impl Write,
    index: &SummaryIndex<'_>,
    header: &str,
    rows: &[DisplayRow],
) -> io::Result<()> {
    write!(
        output,
        "| {header} | Sky | hecs | Bevy | Flecs C | FreeCS | Shipyard |\n\
         |---|---:|---:|---:|---:|---:|---:|\n"
    )?;
    for row in rows {
        write!(output, "| {} |", row.test)?;
        for engine in Engine::ALL {
            write!(output, " {} |", index.cell(row.family, engine))?;
        }
        writeln!(output)?;
    }
    writeln!(output)
}

fn format_duration(nanoseconds: f64) -> String {
    if nanoseconds < 1_000.0 {
        format!("{nanoseconds:.3} ns")
    } else if nanoseconds < 1_000_000.0 {
        format!("{:.3} µs", nanoseconds / 1_000.0)
    } else {
        format!("{:.3} ms", nanoseconds / 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RunEstimate;
    use sky_ecs_comparison::common::BenchmarkClass;
    use sky_ecs_comparison::common::CANONICAL_BENCHMARKS;
    use std::collections::BTreeSet;

    #[test]
    fn fixed_layout_covers_every_canonical_family_once() {
        let layout: BTreeSet<_> = COMPARABLE_ROWS
            .iter()
            .chain(RANDOM_TAG_ROWS.iter())
            .chain(RANDOM_COMPONENT_ROWS.iter())
            .chain(GAMEPLAY_ROWS.iter())
            .chain(DIAGNOSTIC_ROWS.iter())
            .map(|row| row.family)
            .collect();
        let catalog: BTreeSet<_> = CANONICAL_BENCHMARKS
            .iter()
            .map(|spec| spec.family)
            .collect();
        assert_eq!(layout.len(), 37);
        assert_eq!(layout, catalog);
    }

    #[test]
    fn duration_format_uses_readable_units() {
        assert_eq!(format_duration(42.0), "42.000 ns");
        assert_eq!(format_duration(42_000.0), "42.000 µs");
        assert_eq!(format_duration(42_000_000.0), "42.000 ms");
    }

    #[test]
    fn markdown_uses_the_fixed_pivoted_format() {
        let summaries = CANONICAL_BENCHMARKS
            .iter()
            .flat_map(|spec| {
                spec.engines
                    .iter()
                    .enumerate()
                    .map(move |(index, engine)| Summary {
                        benchmark: format!("{}/{}", spec.family, engine.name()),
                        class: spec.class.name().to_owned(),
                        median_ns: (index + 1) as f64 * 1_000.0,
                        work_items: spec.work_items.resolve(),
                        ns_per_item: None,
                        items_per_second: None,
                        plan_payload_bytes: None,
                        amortized_ns_per_traversal: None,
                        run_spread_percent: 0.0,
                        noisy: false,
                        runs: vec![RunEstimate {
                            run: 1,
                            order: Engine::ALL
                                .iter()
                                .map(|engine| engine.name())
                                .collect::<Vec<_>>()
                                .join(","),
                            point_ns: 1_000.0,
                            lower_ns: 900.0,
                            upper_ns: 1_100.0,
                        }],
                    })
            })
            .collect::<Vec<_>>();
        let order_bias = OrderBias {
            available: false,
            reason: Some("test data".to_owned()),
            positions: Vec::new(),
            max_deviation_percent: None,
            spread_percent: None,
            complete: false,
            noisy: true,
        };
        let mut output = Vec::new();
        write_markdown(
            &mut output,
            Some("[GitHub Actions run 123](https://example.test/123)"),
            &summaries,
            &order_bias,
            false,
        )
        .unwrap();
        let markdown = String::from_utf8(output).unwrap();

        assert!(markdown.contains("\n[GitHub Actions run 123](https://example.test/123)\n"));
        assert!(!markdown.contains("Data source:"));
        assert!(markdown.contains("## Comparable"));
        assert!(markdown.contains("## Random Fragmentation"));
        assert!(markdown.contains("## Gameplay Scenario"));
        assert!(markdown.contains("| Entity construction | Individual 10K |"));
        assert!(markdown.contains("| Full frame |"));
        assert!(markdown.contains("N/A"));
        assert!(!markdown.contains("Fixed Sequence"));
        assert!(!markdown.contains("fixed_sequence"));
        assert_eq!(
            CANONICAL_BENCHMARKS
                .iter()
                .filter(|spec| spec.class == BenchmarkClass::GameplayScenario)
                .count(),
            6
        );
    }
}

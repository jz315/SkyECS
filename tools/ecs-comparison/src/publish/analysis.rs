use super::model::{OrderBias, PositionBias, RunEstimate, Summary};
use serde_json::Value;
use sky_ecs_comparison::common::{
    benchmark_class, benchmark_spec, benchmark_work_items, is_canonical_group, BenchmarkClass,
    CANONICAL_BENCHMARKS,
};
use sky_ecs_comparison::Engine;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub(super) fn collect_run(
    benchmark_target: &Path,
    report_dir: &Path,
    run: usize,
    order: &str,
    estimates: &mut BTreeMap<String, Vec<RunEstimate>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let criterion = benchmark_target.join("criterion");
    let raw_dir = report_dir.join(format!("run-{}-raw", run + 1));
    for entry in WalkDir::new(&criterion).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let relative = path.strip_prefix(&criterion)?;
        let Some(group) = relative.components().next() else {
            continue;
        };
        if !is_canonical_group(&group.as_os_str().to_string_lossy()) {
            continue;
        }
        if entry.file_type().is_file() {
            let destination = raw_dir.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, destination)?;
        }

        if path.file_name() != Some(OsStr::new("estimates.json"))
            || path.parent().and_then(Path::file_name) != Some(OsStr::new("new"))
        {
            continue;
        }

        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        let median = &value["median"];
        let benchmark_metadata: Value =
            serde_json::from_slice(&fs::read(path.with_file_name("benchmark.json"))?)?;
        let benchmark = benchmark_metadata["full_id"]
            .as_str()
            .ok_or("Criterion benchmark metadata has no full_id")?
            .to_owned();
        estimates.entry(benchmark).or_default().push(RunEstimate {
            run: run + 1,
            order: order.to_owned(),
            point_ns: number(median, "point_estimate")?,
            lower_ns: number(&median["confidence_interval"], "lower_bound")?,
            upper_ns: number(&median["confidence_interval"], "upper_bound")?,
        });
    }
    Ok(())
}

fn number(value: &Value, field: &str) -> Result<f64, Box<dyn std::error::Error>> {
    value[field]
        .as_f64()
        .ok_or_else(|| format!("missing numeric field `{field}`").into())
}

pub(super) fn summarize(estimates: BTreeMap<String, Vec<RunEstimate>>) -> Vec<Summary> {
    estimates
        .into_iter()
        .map(|(benchmark, runs)| {
            let mut points: Vec<_> = runs.iter().map(|run| run.point_ns).collect();
            points.sort_by(f64::total_cmp);
            let middle = points.len() / 2;
            let median_ns = if points.len().is_multiple_of(2) {
                (points[middle - 1] + points[middle]) * 0.5
            } else {
                points[middle]
            };
            let work_items = benchmark_work_items(&benchmark);
            let minimum = points.first().copied().unwrap_or(median_ns);
            let maximum = points.last().copied().unwrap_or(median_ns);
            let run_spread_percent = if median_ns == 0.0 {
                0.0
            } else {
                (maximum - minimum) * 100.0 / median_ns
            };
            Summary {
                class: benchmark_class(&benchmark)
                    .map(BenchmarkClass::name)
                    .unwrap_or("unknown")
                    .to_owned(),
                benchmark,
                median_ns,
                work_items,
                ns_per_item: work_items.map(|count| median_ns / count as f64),
                items_per_second: work_items.map(|count| count as f64 * 1e9 / median_ns),
                plan_payload_bytes: None,
                amortized_ns_per_traversal: None,
                run_spread_percent,
                noisy: run_spread_percent > 10.0,
                runs,
            }
        })
        .collect()
}

pub(super) fn validate_results(
    summaries: &[Summary],
    run_count: usize,
    require_all_engines: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if summaries.is_empty() {
        return Err("publication produced no benchmark results".into());
    }
    for summary in summaries {
        if summary.runs.len() != run_count {
            return Err(format!(
                "benchmark `{}` has {} runs, expected {run_count}",
                summary.benchmark,
                summary.runs.len()
            )
            .into());
        }
        let expected_class = benchmark_class(&summary.benchmark)
            .ok_or_else(|| format!("benchmark `{}` is not canonical", summary.benchmark))?
            .name();
        if !summary.class.is_empty() && summary.class != expected_class {
            return Err(format!(
                "benchmark `{}` has class `{}`, expected `{expected_class}`",
                summary.benchmark, summary.class
            )
            .into());
        }
    }
    if !require_all_engines {
        return Ok(());
    }

    let mut families: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for summary in summaries {
        let (family, engine) = summary
            .benchmark
            .rsplit_once('/')
            .ok_or_else(|| format!("benchmark `{}` has no engine suffix", summary.benchmark))?;
        if Engine::parse(engine).is_none() {
            return Err(format!(
                "benchmark `{}` has an unknown engine suffix",
                summary.benchmark
            )
            .into());
        }
        families.entry(family).or_default().insert(engine);
    }
    for (family, engines) in &families {
        let spec = benchmark_spec(family)
            .ok_or_else(|| format!("benchmark family `{family}` is not canonical"))?;
        let expected: BTreeSet<_> = spec.engines.iter().map(|engine| engine.name()).collect();
        if engines != &expected {
            return Err(format!(
                "benchmark family `{family}` is incomplete: found {engines:?}, expected {expected:?}"
            )
            .into());
        }
    }
    let found_families: BTreeSet<_> = families.keys().copied().collect();
    let expected_families: BTreeSet<_> = CANONICAL_BENCHMARKS
        .iter()
        .map(|spec| spec.family)
        .collect();
    if found_families != expected_families {
        return Err(format!(
            "canonical benchmark family set differs: found {found_families:?}, expected {expected_families:?}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn analyze_order_bias(summaries: &[Summary]) -> OrderBias {
    let run_orders = summaries
        .first()
        .map(|summary| summary.runs.as_slice())
        .unwrap_or_default();
    if !forms_complete_position_blocks(run_orders) {
        return OrderBias {
            available: false,
            reason: Some(format!(
                "{} rotation(s) do not form complete six-engine position blocks",
                run_orders.len()
            )),
            positions: Vec::new(),
            max_deviation_percent: None,
            spread_percent: None,
            complete: false,
            noisy: true,
        };
    }

    let mut families: BTreeMap<&str, Vec<Vec<f64>>> = BTreeMap::new();
    for summary in summaries {
        let Some((family, engine)) = summary.benchmark.rsplit_once('/') else {
            continue;
        };
        if Engine::parse(engine).is_none() || summary.median_ns <= 0.0 {
            continue;
        }
        if benchmark_class(&summary.benchmark) != Some(BenchmarkClass::Comparable) {
            continue;
        }
        let Some(spec) = benchmark_spec(family) else {
            continue;
        };
        if spec.engines.len() != Engine::ALL.len() {
            continue;
        }
        let positions = families
            .entry(family)
            .or_insert_with(|| vec![Vec::new(); Engine::ALL.len()]);
        for run in &summary.runs {
            let order: Vec<_> = run.order.split(',').collect();
            if let Some(position) = order.iter().position(|candidate| *candidate == engine) {
                if run.point_ns > 0.0 {
                    positions[position].push((run.point_ns / summary.median_ns).ln());
                }
            }
        }
    }

    let mut ratios_by_position = vec![Vec::new(); Engine::ALL.len()];
    let comparable_count = CANONICAL_BENCHMARKS
        .iter()
        .filter(|spec| {
            spec.class == BenchmarkClass::Comparable && spec.engines.len() == Engine::ALL.len()
        })
        .count();
    let mut complete = families.len() == comparable_count;
    for position_logs in families.values() {
        for (position, logs) in position_logs.iter().enumerate() {
            if logs.len() != run_orders.len() {
                complete = false;
                continue;
            }
            ratios_by_position[position].push((logs.iter().sum::<f64>() / logs.len() as f64).exp());
        }
    }

    let raw_ratios: Vec<_> = ratios_by_position
        .iter()
        .map(|ratios| median(ratios.clone()))
        .collect();
    let center = geometric_mean(&raw_ratios);
    let positions: Vec<_> = ratios_by_position
        .into_iter()
        .enumerate()
        .map(|(position, ratios)| PositionBias {
            position: position + 1,
            sample_count: ratios.len(),
            median_ratio: raw_ratios[position] / center,
        })
        .collect();
    let minimum = positions
        .iter()
        .map(|position| position.median_ratio)
        .fold(f64::INFINITY, f64::min);
    let maximum = positions
        .iter()
        .map(|position| position.median_ratio)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_deviation_percent = positions
        .iter()
        .map(|position| (position.median_ratio - 1.0).abs() * 100.0)
        .fold(0.0, f64::max);
    let spread_percent = (maximum - minimum) * 100.0;
    complete &= positions.iter().all(|position| {
        position.sample_count == comparable_count && position.median_ratio.is_finite()
    });
    OrderBias {
        available: complete,
        reason: (!complete).then(|| {
            "comparable workload coverage is incomplete for at least one position".to_owned()
        }),
        positions,
        max_deviation_percent: complete.then_some(max_deviation_percent),
        spread_percent: complete.then_some(spread_percent),
        complete,
        noisy: !complete || max_deviation_percent > 3.0 || spread_percent > 5.0,
    }
}

fn forms_complete_position_blocks(runs: &[RunEstimate]) -> bool {
    if runs.is_empty() || !runs.len().is_multiple_of(Engine::ALL.len()) {
        return false;
    }
    runs.chunks_exact(Engine::ALL.len()).all(|block| {
        Engine::ALL.iter().all(|engine| {
            let mut positions = BTreeSet::new();
            for run in block {
                let order: Vec<_> = run.order.split(',').collect();
                let Some(position) = order
                    .iter()
                    .position(|candidate| *candidate == engine.name())
                else {
                    return false;
                };
                positions.insert(position);
            }
            positions.len() == Engine::ALL.len()
        })
    })
}

fn geometric_mean(values: &[f64]) -> f64 {
    if values.is_empty()
        || values
            .iter()
            .any(|value| *value <= 0.0 || !value.is_finite())
    {
        return 1.0;
    }
    (values.iter().map(|value| value.ln()).sum::<f64>() / values.len() as f64).exp()
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::rotated_order;

    #[test]
    fn median_handles_odd_and_even_inputs() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn order_bias_detects_a_shared_position_penalty() {
        let mut summaries = Vec::new();
        for spec in CANONICAL_BENCHMARKS {
            for &engine in spec.engines {
                let runs = (0..Engine::ALL.len())
                    .map(|run| {
                        let order = rotated_order(run);
                        let position = order
                            .split(',')
                            .position(|candidate| candidate == engine.name())
                            .expect("engine must occur in every rotation");
                        let point_ns = if position == 0 { 1.2 } else { 1.0 };
                        RunEstimate {
                            run: run + 1,
                            order,
                            point_ns,
                            lower_ns: point_ns,
                            upper_ns: point_ns,
                        }
                    })
                    .collect();
                summaries.push(Summary {
                    benchmark: format!("{}/{}", spec.family, engine.name()),
                    class: spec.class.name().to_owned(),
                    median_ns: 1.0,
                    work_items: None,
                    ns_per_item: None,
                    items_per_second: None,
                    plan_payload_bytes: None,
                    amortized_ns_per_traversal: None,
                    run_spread_percent: 20.0,
                    noisy: true,
                    runs,
                });
            }
        }

        let bias = analyze_order_bias(&summaries);
        assert!(bias.complete);
        let comparable_count = CANONICAL_BENCHMARKS
            .iter()
            .filter(|spec| {
                spec.class == BenchmarkClass::Comparable && spec.engines.len() == Engine::ALL.len()
            })
            .count();
        assert_eq!(bias.positions[0].sample_count, comparable_count);
        assert!(bias.positions[0].median_ratio > 1.15);
        assert!(bias.max_deviation_percent.unwrap() > 15.0);
        assert!(bias.noisy);
    }

    #[test]
    fn four_rotations_report_order_bias_as_unavailable() {
        let spec = CANONICAL_BENCHMARKS
            .iter()
            .find(|spec| spec.class == BenchmarkClass::Comparable)
            .unwrap();
        let summaries = spec
            .engines
            .iter()
            .map(|engine| Summary {
                benchmark: format!("{}/{}", spec.family, engine.name()),
                class: spec.class.name().to_owned(),
                median_ns: 1.0,
                work_items: None,
                ns_per_item: None,
                items_per_second: None,
                plan_payload_bytes: None,
                amortized_ns_per_traversal: None,
                run_spread_percent: 0.0,
                noisy: false,
                runs: (0..4)
                    .map(|run| RunEstimate {
                        run: run + 1,
                        order: rotated_order(run),
                        point_ns: 1.0,
                        lower_ns: 1.0,
                        upper_ns: 1.0,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();

        let bias = analyze_order_bias(&summaries);
        assert!(!bias.available);
        assert!(bias.positions.is_empty());
        assert_eq!(bias.max_deviation_percent, None);
        assert!(bias.reason.unwrap().contains("4 rotation"));
    }

    #[test]
    fn result_validation_accepts_the_canonical_support_matrix() {
        let summaries = CANONICAL_BENCHMARKS
            .iter()
            .flat_map(|spec| {
                spec.engines.iter().map(|engine| Summary {
                    benchmark: format!("{}/{}", spec.family, engine.name()),
                    class: spec.class.name().to_owned(),
                    median_ns: 1.0,
                    work_items: None,
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
                        point_ns: 1.0,
                        lower_ns: 1.0,
                        upper_ns: 1.0,
                    }],
                })
            })
            .collect::<Vec<_>>();

        validate_results(&summaries, 1, true).unwrap();
    }
}

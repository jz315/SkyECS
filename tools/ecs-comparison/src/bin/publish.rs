use serde::{Deserialize, Serialize};
use serde_json::Value;
use sky_ecs_comparison::common::{random_fragment_masks, random_fragment_match_count};
use sky_ecs_comparison::Engine;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const DEFAULT_RUN_COUNT: usize = 7;
const EXPECTED_CANONICAL_FAMILIES: [&str; 23] = [
    "fair_prepared_construction/bulk_insert_10k",
    "fair_prepared_construction/single_insert_10k",
    "fair_prepared_iteration/simple_10k",
    "fair_prepared_iteration_repeated/simple_x32",
    "fair_prepared_iteration_large/simple_100k",
    "fair_prepared_fragmented_iteration/fragmented_26x400",
    "fair_prepared_random_fragmented_iteration/random_6_components_4_terms",
    "fair_prepared_random_fragmented_iteration/random_8_components_4_terms",
    "fair_prepared_random_fragmented_iteration/random_10_components_4_terms",
    "fair_prepared_random_fragmented_iteration/random_16_components_4_terms",
    "fair_diagnostic_heavy_compute/heavy",
    "fair_prepared_random_access/hot_10k",
    "fair_prepared_random_access/warm_100k",
    "fair_prepared_random_access/cold_1m",
    "fair_entity_ops/spawn_despawn_1k",
    "fair_entity_ops/add_remove_component_1k",
    "fair_scenario_mixed_frame/frame",
    "fair_scenario_mixed_frame_phases/movement",
    "fair_scenario_mixed_frame_phases/health",
    "fair_scenario_mixed_frame_phases/heavy",
    "fair_scenario_mixed_frame_phases/random_access",
    "fair_scenario_mixed_frame_phases/structural_churn",
    "fair_scenario_mixed_frame_phases/spawn_despawn",
];
struct Options {
    runs: usize,
    filter: Option<String>,
    reanalyze: Option<PathBuf>,
}

#[derive(Clone, Deserialize, Serialize)]
struct RunEstimate {
    run: usize,
    order: String,
    point_ns: f64,
    lower_ns: f64,
    upper_ns: f64,
}

#[derive(Deserialize, Serialize)]
struct Summary {
    benchmark: String,
    median_ns: f64,
    work_items: Option<usize>,
    ns_per_item: Option<f64>,
    items_per_second: Option<f64>,
    run_spread_percent: f64,
    noisy: bool,
    runs: Vec<RunEstimate>,
}

#[derive(Serialize)]
struct PositionBias {
    position: usize,
    sample_count: usize,
    median_ratio: f64,
}

#[derive(Serialize)]
struct OrderBias {
    positions: Vec<PositionBias>,
    max_deviation_percent: f64,
    spread_percent: f64,
    complete: bool,
    noisy: bool,
}

#[derive(Serialize)]
struct PublicationReport<'a> {
    criterion_estimator: &'static str,
    run_count: usize,
    order_bias: &'a OrderBias,
    benchmarks: &'a [Summary],
}

#[derive(Deserialize)]
struct StoredPublicationReport {
    run_count: usize,
    benchmarks: Vec<Summary>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options()?;
    if let Some(report_dir) = options.reanalyze {
        return reanalyze_report(&report_dir);
    }
    let root = workspace_root()?;
    let benchmark_target = root.join("target/fair-publish-target");
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let report_dir = root.join("target/fair-reports").join(stamp.to_string());
    fs::create_dir_all(&report_dir)?;
    write_metadata(&root, &report_dir)?;

    let mut estimates: BTreeMap<String, Vec<RunEstimate>> = BTreeMap::new();
    for run in 0..options.runs {
        let order = rotated_order(run);
        println!(
            "compare-ecs publication run {}/{}: {order}",
            run + 1,
            options.runs
        );
        clear_v2_results(&benchmark_target)?;
        run_bench(
            &root,
            &benchmark_target,
            &report_dir,
            run,
            &order,
            options.filter.as_deref(),
        )?;
        collect_run(&benchmark_target, &report_dir, run, &order, &mut estimates)?;
    }

    let summaries = summarize(estimates);
    validate_results(&summaries, options.runs, options.filter.is_none())?;
    write_report(&report_dir, options.runs, &summaries)?;
    println!("publication report: {}", report_dir.display());
    Ok(())
}

fn write_report(
    report_dir: &Path,
    run_count: usize,
    summaries: &[Summary],
) -> Result<(), Box<dyn std::error::Error>> {
    let order_bias = analyze_order_bias(summaries);
    let report = PublicationReport {
        criterion_estimator: "median",
        run_count,
        order_bias: &order_bias,
        benchmarks: summaries,
    };
    fs::write(
        report_dir.join("summary.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    write_markdown(report_dir, run_count, summaries, &order_bias)?;
    Ok(())
}

fn reanalyze_report(report_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stored: StoredPublicationReport =
        serde_json::from_slice(&fs::read(report_dir.join("summary.json"))?)?;
    validate_results(&stored.benchmarks, stored.run_count, true)?;
    write_report(report_dir, stored.run_count, &stored.benchmarks)?;
    println!("reanalyzed publication report: {}", report_dir.display());
    Ok(())
}

fn clear_v2_results(benchmark_target: &Path) -> io::Result<()> {
    let criterion = benchmark_target.join("criterion");
    let Ok(entries) = fs::read_dir(criterion) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() && is_v2_group(&entry.file_name().to_string_lossy()) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut runs_explicit = false;
    let mut options = Options {
        runs: DEFAULT_RUN_COUNT,
        filter: None,
        reanalyze: None,
    };
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--runs" => {
                runs_explicit = true;
                options.runs = args.next().ok_or("--runs requires a value")?.parse()?;
                if !(1..=Engine::ALL.len()).contains(&options.runs) {
                    return Err("--runs must be between 1 and 7".into());
                }
            }
            "--filter" => options.filter = Some(args.next().ok_or("--filter requires a value")?),
            "--reanalyze" => {
                options.reanalyze = Some(PathBuf::from(
                    args.next().ok_or("--reanalyze requires a report path")?,
                ));
            }
            _ => return Err(format!("unknown argument `{argument}`").into()),
        }
    }
    if options.reanalyze.is_some() && (options.filter.is_some() || runs_explicit) {
        return Err("--reanalyze cannot be combined with --runs or --filter".into());
    }
    Ok(options)
}

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new(cargo())
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()?;
    if !output.status.success() {
        return Err("cargo locate-project failed".into());
    }
    let manifest = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    Ok(manifest
        .parent()
        .ok_or("workspace manifest has no parent")?
        .to_path_buf())
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn rotated_order(offset: usize) -> String {
    (0..Engine::ALL.len())
        .map(|index| Engine::ALL[(index + offset) % Engine::ALL.len()].name())
        .collect::<Vec<_>>()
        .join(",")
}

fn run_bench(
    root: &Path,
    benchmark_target: &Path,
    report_dir: &Path,
    run: usize,
    order: &str,
    filter: Option<&str>,
) -> io::Result<()> {
    let log_path = report_dir.join(format!("run-{}.log", run + 1));
    let log = File::create(log_path)?;
    let error_log = log.try_clone()?;
    let mut command = Command::new(cargo());
    command
        .current_dir(root)
        .args(["bench", "-p", "sky_ecs_comparison", "--bench", "fair", "--"]);
    if let Some(filter) = filter {
        command.arg(filter);
    }
    let status = command
        .arg("--noplot")
        .env("CARGO_TARGET_DIR", benchmark_target)
        .env("SKY_ECS_ORDER", order)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log))
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "publication run {} failed; inspect its log",
            run + 1
        )));
    }
    Ok(())
}

fn native_optimized_flags() -> &'static str {
    if cfg!(target_env = "msvc") {
        "/O2 /GL /LTCG /DNDEBUG"
    } else {
        "-O3 -flto -DNDEBUG"
    }
}

fn collect_run(
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
        if !is_v2_group(&group.as_os_str().to_string_lossy()) {
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

fn is_v2_group(group: &str) -> bool {
    matches!(
        group,
        "fair_prepared_construction"
            | "fair_prepared_iteration"
            | "fair_prepared_iteration_repeated"
            | "fair_prepared_iteration_large"
            | "fair_prepared_fragmented_iteration"
            | "fair_prepared_random_fragmented_iteration"
            | "fair_diagnostic_heavy_compute"
            | "fair_prepared_random_access"
            | "fair_entity_ops"
            | "fair_scenario_mixed_frame"
            | "fair_scenario_mixed_frame_phases"
    )
}

fn summarize(estimates: BTreeMap<String, Vec<RunEstimate>>) -> Vec<Summary> {
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
            let work_items = work_items(&benchmark);
            let minimum = points.first().copied().unwrap_or(median_ns);
            let maximum = points.last().copied().unwrap_or(median_ns);
            let run_spread_percent = if median_ns == 0.0 {
                0.0
            } else {
                (maximum - minimum) * 100.0 / median_ns
            };
            Summary {
                benchmark,
                median_ns,
                work_items,
                ns_per_item: work_items.map(|count| median_ns / count as f64),
                items_per_second: work_items.map(|count| count as f64 * 1e9 / median_ns),
                run_spread_percent,
                noisy: run_spread_percent > 10.0,
                runs,
            }
        })
        .collect()
}

fn work_items(benchmark: &str) -> Option<usize> {
    if benchmark.contains("cold_1m") {
        Some(1_000_000)
    } else if benchmark.contains("warm_100k") || benchmark.contains("simple_100k") {
        Some(100_000)
    } else if benchmark.contains("fragmented_26x400") {
        Some(10_400)
    } else if benchmark.contains("random_") && benchmark.contains("_components_4_terms") {
        let component_count = benchmark
            .rsplit_once("/random_")
            .map(|(_, suffix)| suffix)
            .and_then(|suffix| suffix.split("_components_4_terms").next())
            .and_then(|value| value.parse().ok())?;
        Some(random_fragment_match_count(&random_fragment_masks(
            component_count,
        )))
    } else if benchmark.contains("simple_x32") {
        Some(320_000)
    } else if benchmark.contains("10k")
        || benchmark.contains("hot_10k")
        || benchmark.contains("get_10000")
    {
        Some(10_000)
    } else if benchmark.contains("get_16") {
        Some(16)
    } else if benchmark.contains("get_1/") || benchmark.contains("get_1_") {
        Some(1)
    } else if benchmark.contains("_1k") {
        Some(1_000)
    } else {
        None
    }
}

fn write_markdown(
    report_dir: &Path,
    run_count: usize,
    summaries: &[Summary],
    order_bias: &OrderBias,
) -> io::Result<()> {
    let mut output = File::create(report_dir.join("summary.md"))?;
    writeln!(output, "# Compare-ECS publication report\n")?;
    writeln!(
        output,
        "{run_count} Latin-square run(s); values are medians of the per-run Criterion median estimates.\n"
    )?;
    writeln!(output, "| Benchmark | Median | ns/item | items/s |")?;
    writeln!(output, "|---|---:|---:|---:|")?;
    for summary in summaries {
        writeln!(
            output,
            "| `{}` | {:.3} µs | {} | {} |",
            summary.benchmark,
            summary.median_ns / 1_000.0,
            summary
                .ns_per_item
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "—".to_owned()),
            summary
                .items_per_second
                .map(|value| format!("{value:.0}"))
                .unwrap_or_else(|| "—".to_owned())
        )?;
    }
    writeln!(output, "\n## Order-bias check\n")?;
    writeln!(
        output,
        "Each position is the centered median of per-workload geometric means across all seven engines.\n"
    )?;
    writeln!(output, "| Position | Workloads | Median normalized time |")?;
    writeln!(output, "|---:|---:|---:|")?;
    for position in &order_bias.positions {
        writeln!(
            output,
            "| {} | {} | {:.4}× |",
            position.position, position.sample_count, position.median_ratio
        )?;
    }
    let order_status = if !order_bias.complete {
        "insufficient data"
    } else if order_bias.noisy {
        "noisy"
    } else {
        "stable"
    };
    writeln!(
        output,
        "\nMaximum deviation: {:.2}%; position spread: {:.2}%; status: **{}**.",
        order_bias.max_deviation_percent, order_bias.spread_percent, order_status
    )?;

    let noisy: Vec<_> = summaries.iter().filter(|summary| summary.noisy).collect();
    if !noisy.is_empty() {
        writeln!(output, "\n## Noisy benchmarks\n")?;
        for summary in noisy {
            writeln!(
                output,
                "- `{}`: {:.2}% max-to-min spread across runs",
                summary.benchmark, summary.run_spread_percent
            )?;
        }
    }
    Ok(())
}

fn validate_results(
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
    let expected: BTreeSet<_> = Engine::ALL.iter().map(|engine| engine.name()).collect();
    for (family, engines) in &families {
        if engines != &expected {
            return Err(format!(
                "benchmark family `{family}` is incomplete: found {engines:?}, expected {expected:?}"
            )
            .into());
        }
    }
    let found_families: BTreeSet<_> = families.keys().copied().collect();
    let expected_families: BTreeSet<_> = EXPECTED_CANONICAL_FAMILIES.into_iter().collect();
    if found_families != expected_families {
        return Err(format!(
            "canonical benchmark family set differs: found {found_families:?}, expected {expected_families:?}"
        )
        .into());
    }
    Ok(())
}

fn analyze_order_bias(summaries: &[Summary]) -> OrderBias {
    let mut families: BTreeMap<&str, Vec<Vec<f64>>> = BTreeMap::new();
    for summary in summaries {
        let Some((family, engine)) = summary.benchmark.rsplit_once('/') else {
            continue;
        };
        if Engine::parse(engine).is_none() || summary.median_ns <= 0.0 {
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

    // A cyclic Latin square puts every engine in every position. First combine
    // the seven engines within one workload using a geometric mean: this
    // balances run-to-run machine drift because every position contains every
    // run once. Then take the median across workloads so one noisy workload
    // cannot dominate the order check.
    let mut ratios_by_position = vec![Vec::new(); Engine::ALL.len()];
    let mut complete = families.len() == EXPECTED_CANONICAL_FAMILIES.len();
    for position_logs in families.values() {
        for (position, logs) in position_logs.iter().enumerate() {
            if logs.len() != Engine::ALL.len() {
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
        position.sample_count == EXPECTED_CANONICAL_FAMILIES.len()
            && position.median_ratio.is_finite()
    });
    OrderBias {
        positions,
        max_deviation_percent,
        spread_percent,
        complete,
        noisy: !complete || max_deviation_percent > 3.0 || spread_percent > 5.0,
    }
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

fn write_metadata(root: &Path, report_dir: &Path) -> io::Result<()> {
    let mut output = File::create(report_dir.join("environment.txt"))?;
    writeln!(output, "os={} arch={}", env::consts::OS, env::consts::ARCH)?;
    writeln!(output, "criterion_estimator=median")?;
    writeln!(output, "rust_bench_lto=fat codegen_units=1 opt_level=3")?;
    writeln!(output, "native_c_flags={}", native_optimized_flags())?;
    writeln!(
        output,
        "flecs_cpp_core_version={} commit={}",
        env!("SKY_FLECS_CPP_CORE_VERSION"),
        env!("SKY_FLECS_CPP_CORE_COMMIT")
    )?;
    writeln!(
        output,
        "flecs_rust_core_version={}",
        env!("SKY_FLECS_RUST_CORE_VERSION")
    )?;
    writeln!(
        output,
        "processor={}",
        env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_owned())
    )?;
    for (program, args) in [
        ("rustc", vec!["-Vv"]),
        ("git", vec!["rev-parse", "HEAD"]),
        ("git", vec!["status", "--short"]),
        (
            "cargo",
            vec!["tree", "-p", "sky_ecs_comparison", "--depth", "1"],
        ),
    ] {
        let result = Command::new(program)
            .current_dir(root)
            .args(args)
            .output()?;
        output.write_all(&result.stdout)?;
        output.write_all(&result.stderr)?;
        writeln!(output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_put_every_engine_in_every_position() {
        let mut positions: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for offset in 0..Engine::ALL.len() {
            for (position, engine) in rotated_order(offset).split(',').enumerate() {
                positions
                    .entry(engine.to_owned())
                    .or_default()
                    .insert(position);
            }
        }
        for engine in Engine::ALL {
            assert_eq!(positions[engine.name()].len(), Engine::ALL.len());
        }
    }

    #[test]
    fn work_item_count_uses_random_fragment_component_count() {
        for component_count in [6, 8, 10, 16] {
            let benchmark = format!(
                "fair_prepared_random_fragmented_iteration/random_{component_count}_components_4_terms/sky"
            );
            assert_eq!(
                work_items(&benchmark),
                Some(random_fragment_match_count(&random_fragment_masks(
                    component_count
                )))
            );
        }
    }

    #[test]
    fn median_handles_odd_and_even_inputs() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn order_bias_detects_a_shared_position_penalty() {
        let mut summaries = Vec::new();
        for family in EXPECTED_CANONICAL_FAMILIES {
            for engine in Engine::ALL {
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
                    benchmark: format!("{family}/{}", engine.name()),
                    median_ns: 1.0,
                    work_items: None,
                    ns_per_item: None,
                    items_per_second: None,
                    run_spread_percent: 20.0,
                    noisy: true,
                    runs,
                });
            }
        }

        let bias = analyze_order_bias(&summaries);
        assert!(bias.complete);
        assert_eq!(
            bias.positions[0].sample_count,
            EXPECTED_CANONICAL_FAMILIES.len()
        );
        assert!(bias.positions[0].median_ratio > 1.15);
        assert!(bias.max_deviation_percent > 15.0);
        assert!(bias.noisy);
    }

    #[test]
    fn publication_protocol_has_unique_expected_families() {
        assert_eq!(
            EXPECTED_CANONICAL_FAMILIES
                .into_iter()
                .collect::<BTreeSet<_>>()
                .len(),
            EXPECTED_CANONICAL_FAMILIES.len()
        );
    }
}

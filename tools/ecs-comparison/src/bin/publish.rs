use serde::Serialize;
use serde_json::Value;
use sky_ecs_comparison::Engine;
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const DEFAULT_RUN_COUNT: usize = 6;

struct Options {
    runs: usize,
    filter: Option<String>,
}

#[derive(Clone, Serialize)]
struct RunEstimate {
    run: usize,
    order: String,
    point_ns: f64,
    lower_ns: f64,
    upper_ns: f64,
}

#[derive(Serialize)]
struct Summary {
    benchmark: String,
    median_ns: f64,
    work_items: Option<usize>,
    ns_per_item: Option<f64>,
    items_per_second: Option<f64>,
    runs: Vec<RunEstimate>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options()?;
    let root = workspace_root()?;
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
        clear_v2_results(&root)?;
        run_bench(&root, &report_dir, run, &order, options.filter.as_deref())?;
        collect_run(&root, &report_dir, run, &order, &mut estimates)?;
    }

    let summaries = summarize(estimates);
    fs::write(
        report_dir.join("summary.json"),
        serde_json::to_vec_pretty(&summaries)?,
    )?;
    write_markdown(&report_dir, options.runs, &summaries)?;
    println!("publication report: {}", report_dir.display());
    Ok(())
}

fn clear_v2_results(root: &Path) -> io::Result<()> {
    let criterion = root.join("target/criterion");
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
    let mut options = Options {
        runs: DEFAULT_RUN_COUNT,
        filter: None,
    };
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--runs" => {
                options.runs = args.next().ok_or("--runs requires a value")?.parse()?;
                if !(1..=Engine::ALL.len()).contains(&options.runs) {
                    return Err("--runs must be between 1 and 6".into());
                }
            }
            "--filter" => options.filter = Some(args.next().ok_or("--filter requires a value")?),
            _ => return Err(format!("unknown argument `{argument}`").into()),
        }
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

fn collect_run(
    root: &Path,
    report_dir: &Path,
    run: usize,
    order: &str,
    estimates: &mut BTreeMap<String, Vec<RunEstimate>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let criterion = root.join("target/criterion");
    let raw_dir = report_dir.join(format!("run-{}-raw", run + 1));
    for entry in WalkDir::new(&criterion).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.file_name() != Some(OsStr::new("estimates.json"))
            || path.parent().and_then(Path::file_name) != Some(OsStr::new("new"))
        {
            continue;
        }
        let relative = path.strip_prefix(&criterion)?;
        let Some(group) = relative.components().next() else {
            continue;
        };
        if !is_v2_group(&group.as_os_str().to_string_lossy()) {
            continue;
        }
        let destination = raw_dir.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, destination)?;

        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        let mean = &value["mean"];
        let benchmark = relative
            .parent()
            .and_then(Path::parent)
            .ok_or("invalid Criterion result path")?
            .to_string_lossy()
            .replace('\\', "/");
        estimates.entry(benchmark).or_default().push(RunEstimate {
            run: run + 1,
            order: order.to_owned(),
            point_ns: number(mean, "point_estimate")?,
            lower_ns: number(&mean["confidence_interval"], "lower_bound")?,
            upper_ns: number(&mean["confidence_interval"], "upper_bound")?,
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
        "fair_construction"
            | "fair_prepared_iteration"
            | "fair_prepared_iteration_repeated"
            | "fair_prepared_iteration_large"
            | "fair_prepared_fragmented_iteration"
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
            let median_ns = if points.len() % 2 == 0 {
                (points[middle - 1] + points[middle]) * 0.5
            } else {
                points[middle]
            };
            let work_items = work_items(&benchmark);
            Summary {
                benchmark,
                median_ns,
                work_items,
                ns_per_item: work_items.map(|count| median_ns / count as f64),
                items_per_second: work_items.map(|count| count as f64 * 1e9 / median_ns),
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

fn write_markdown(report_dir: &Path, run_count: usize, summaries: &[Summary]) -> io::Result<()> {
    let mut output = File::create(report_dir.join("summary.md"))?;
    writeln!(output, "# Compare-ECS publication report\n")?;
    writeln!(
        output,
        "{run_count} Latin-square run(s); values are cross-run medians.\n"
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
    Ok(())
}

fn write_metadata(root: &Path, report_dir: &Path) -> io::Result<()> {
    let mut output = File::create(report_dir.join("environment.txt"))?;
    writeln!(output, "os={} arch={}", env::consts::OS, env::consts::ARCH)?;
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

use super::analysis::{analyze_order_bias, validate_results};
use super::model::{OrderBias, PublicationReport, StoredPublicationReport, Summary};
use sky_ecs_comparison::common::{benchmark_class, BenchmarkClass};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write_report(
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

pub(super) fn reanalyze_report(report_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut stored: StoredPublicationReport =
        serde_json::from_slice(&fs::read(report_dir.join("summary.json"))?)?;
    for summary in &mut stored.benchmarks {
        if summary.class.is_empty() {
            summary.class = benchmark_class(&summary.benchmark)
                .map(BenchmarkClass::name)
                .unwrap_or("unknown")
                .to_owned();
        }
    }
    validate_results(&stored.benchmarks, stored.run_count, true)?;
    write_report(report_dir, stored.run_count, &stored.benchmarks)?;
    println!("reanalyzed publication report: {}", report_dir.display());
    Ok(())
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
        "{run_count} engine-order rotation(s); values are medians of the per-run Criterion median estimates.\n"
    )?;
    writeln!(
        output,
        "Comparable workloads are used for cross-ECS comparisons. Scenarios and diagnostics are reported separately and are not included in comparative win counts or the order-bias check.\n"
    )?;
    write_summary_table(
        &mut output,
        "Comparable workloads",
        summaries,
        BenchmarkClass::Comparable,
    )?;
    write_summary_table(
        &mut output,
        "Scenario workloads",
        summaries,
        BenchmarkClass::Scenario,
    )?;
    write_summary_table(
        &mut output,
        "Diagnostic workloads",
        summaries,
        BenchmarkClass::Diagnostic,
    )?;
    writeln!(output, "## Order-bias check\n")?;
    if !order_bias.available {
        writeln!(
            output,
            "**N/A.** {}.\n",
            order_bias
                .reason
                .as_deref()
                .unwrap_or("complete position-balanced data is unavailable")
        )?;
        return write_noisy_benchmarks(&mut output, summaries);
    }
    writeln!(
        output,
        "Only comparable workloads supported by all six engines are included. Each position is the centered median of per-workload geometric means across all six engines.\n"
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
        order_bias.max_deviation_percent.unwrap_or_default(),
        order_bias.spread_percent.unwrap_or_default(),
        order_status
    )?;

    write_noisy_benchmarks(&mut output, summaries)
}

fn write_noisy_benchmarks(output: &mut File, summaries: &[Summary]) -> io::Result<()> {
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

fn write_summary_table(
    output: &mut File,
    title: &str,
    summaries: &[Summary],
    class: BenchmarkClass,
) -> io::Result<()> {
    writeln!(output, "## {title}\n")?;
    writeln!(
        output,
        "| Benchmark | Median | ns/item | items/s | Plan payload | Amortized/traversal |"
    )?;
    writeln!(output, "|---|---:|---:|---:|---:|---:|")?;
    for summary in summaries {
        if benchmark_class(&summary.benchmark) != Some(class) {
            continue;
        }
        writeln!(
            output,
            "| `{}` | {:.3} µs | {} | {} | {} | {} |",
            summary.benchmark,
            summary.median_ns / 1_000.0,
            summary
                .ns_per_item
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "—".to_owned()),
            summary
                .items_per_second
                .map(|value| format!("{value:.0}"))
                .unwrap_or_else(|| "—".to_owned()),
            summary
                .plan_payload_bytes
                .map(|bytes| format!("{bytes} B"))
                .unwrap_or_else(|| "—".to_owned()),
            summary
                .amortized_ns_per_traversal
                .map(|value| format!("{:.3} µs", value / 1_000.0))
                .unwrap_or_else(|| "—".to_owned())
        )?;
    }
    writeln!(output)?;
    Ok(())
}

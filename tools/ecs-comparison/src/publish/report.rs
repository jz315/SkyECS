use super::analysis::{analyze_order_bias, validate_results};
use super::layout;
use super::model::{ContractVerification, PublicationReport, StoredPublicationReport, Summary};
use sky_ecs_comparison::common::{benchmark_class, BenchmarkClass};
use std::env;
use std::fs::{self, File};
use std::path::Path;

pub(super) fn write_report(
    report_dir: &Path,
    run_count: usize,
    summaries: &[Summary],
    contracts: &ContractVerification,
    allow_dirty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let order_bias = analyze_order_bias(summaries);
    let report = PublicationReport {
        reproducible: !allow_dirty,
        working_tree_dirty: allow_dirty,
        contracts,
        criterion_estimator: "median",
        run_count,
        order_bias: &order_bias,
        benchmarks: summaries,
    };
    fs::write(
        report_dir.join("summary.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;

    let source = report_source(report_dir);
    let mut output = File::create(report_dir.join("summary.md"))?;
    layout::write_markdown(
        &mut output,
        source.as_deref(),
        summaries,
        &order_bias,
        allow_dirty,
    )?;
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
    write_report(
        report_dir,
        stored.run_count,
        &stored.benchmarks,
        &stored.contracts,
        stored.working_tree_dirty,
    )?;
    println!("reanalyzed publication report: {}", report_dir.display());
    Ok(())
}

fn report_source(report_dir: &Path) -> Option<String> {
    github_source(
        env::var("GITHUB_SERVER_URL").ok().as_deref(),
        env::var("GITHUB_REPOSITORY").ok().as_deref(),
        env::var("GITHUB_RUN_ID").ok().as_deref(),
    )
    .or_else(|| {
        let runner = fs::read_to_string(report_dir.join("github-runner.txt")).ok()?;
        let value = |key: &str| {
            runner
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
        };
        github_source(
            Some("https://github.com"),
            value("github_repository"),
            value("github_run_id"),
        )
    })
}

fn github_source(
    server: Option<&str>,
    repository: Option<&str>,
    run_id: Option<&str>,
) -> Option<String> {
    let server = server?;
    let repository = repository?;
    let run_id = run_id?;
    Some(format!(
        "[GitHub Actions run {run_id}]({server}/{repository}/actions/runs/{run_id})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_source_is_one_link() {
        assert_eq!(
            github_source(
                Some("https://github.com"),
                Some("jz315/SkyECS"),
                Some("123")
            ),
            Some(
                "[GitHub Actions run 123](https://github.com/jz315/SkyECS/actions/runs/123)"
                    .to_owned()
            )
        );
    }
}

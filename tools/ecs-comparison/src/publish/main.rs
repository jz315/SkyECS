mod analysis;
mod metadata;
mod model;
mod options;
mod report;
mod runner;

use analysis::{collect_run, summarize, validate_results};
use metadata::write_metadata;
use model::RunEstimate;
use options::options;
use report::{reanalyze_report, write_report};
use runner::{
    clear_results, ensure_clean_worktree, rotated_order, run_bench, run_contracts, workspace_root,
};
use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options()?;
    if let Some(report_dir) = options.reanalyze {
        return reanalyze_report(&report_dir);
    }
    let root = workspace_root()?;
    if !options.allow_dirty {
        ensure_clean_worktree(&root)?;
    }
    let benchmark_target = root.join("target/comparison-publish-target");
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let report_dir = root
        .join("target/comparison-reports")
        .join(stamp.to_string());
    fs::create_dir_all(&report_dir)?;
    let contracts = run_contracts(&root, &report_dir)?;
    write_metadata(&root, &report_dir)?;

    let mut estimates: BTreeMap<String, Vec<RunEstimate>> = BTreeMap::new();
    for run in 0..options.runs {
        let order = rotated_order(run);
        println!(
            "compare-ecs publication run {}/{}: {order}",
            run + 1,
            options.runs
        );
        clear_results(&benchmark_target)?;
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
    write_report(
        &report_dir,
        options.runs,
        &summaries,
        &contracts,
        options.allow_dirty,
    )?;
    println!("publication report: {}", report_dir.display());
    Ok(())
}
